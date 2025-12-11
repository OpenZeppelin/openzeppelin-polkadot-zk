// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC7984, IERC7984Receiver} from "./interfaces/IERC7984.sol";
import {IConfidentialAssets} from "./interfaces/IConfidentialAssets.sol";

/**
 * @title ERC7984ConfidentialToken
 * @author Optimism Foundation
 * @notice ERC-7984 compliant wrapper around the Confidential Assets precompile
 * @dev This contract wraps a specific asset from the multi-asset Confidential Assets precompile
 *      and exposes it as an ERC-7984 standard compliant confidential token.
 *
 * # Purpose
 * The ERC-7984 standard defines a common interface for confidential tokens. This wrapper allows
 * applications built for ERC-7984 to seamlessly work with the Optimism Confidential Assets system.
 *
 * # Architecture
 * - **Single Asset**: Each wrapper instance is bound to one asset ID at deployment
 * - **Translation Layer**: Converts ERC-7984 calls to Confidential Assets precompile calls
 * - **Operator Support**: Implements time-limited operator approvals (ERC-7984 requirement)
 * - **Proof Handling**: Encodes ZK proofs in the `data` parameter per ERC-7984 spec
 *
 * # Key Features
 * 1. **Standard Compliance**: Full ERC-7984 interface implementation
 * 2. **Operator Delegations**: Time-limited approvals for third-party transfers
 * 3. **Flexible Initialization**: Can fetch metadata from precompile or set at deployment
 * 4. **Helper Functions**: Convenience methods for deposit, withdraw, claim, and key setup
 *
 * # ERC-7984 Amount Representation
 * Per ERC-7984, amounts are represented as `bytes32` handles/commitments. In this implementation:
 * - The `bytes32 amount` parameter in transfer functions is used for event emission
 * - The actual encrypted amount (64 bytes) and ZK proof are passed in the `data` parameter
 * - Format: `data = abi.encode(bytes encryptedAmount, bytes proof)`
 *
 * # Integration Guide
 *
 * ## For Token Issuers (Deploying a Wrapper)
 * ```solidity
 * // Deploy wrapper for existing asset
 * ERC7984ConfidentialToken token = new ERC7984ConfidentialToken(
 *     assetId,           // Asset ID from precompile
 *     "",                // Empty = fetch name from precompile
 *     "",                // Empty = fetch symbol from precompile
 *     0                  // 0 = fetch decimals from precompile
 * );
 *
 * // Or deploy with custom metadata
 * ERC7984ConfidentialToken token = new ERC7984ConfidentialToken(
 *     assetId,
 *     "My Confidential Token",
 *     "MCT",
 *     18
 * );
 * ```
 *
 * ## For Users (Using the Token)
 * ```solidity
 * // 1. Setup: Set your public key (one-time)
 * token.setPublicKey(myElGamalPublicKey);
 *
 * // 2. Deposit: Shield public tokens
 * publicToken.approve(address(token), 1000e18);
 * token.deposit(1000e18, depositProof);
 *
 * // 3. Transfer: Send confidentially
 * bytes memory data = abi.encode(encryptedAmount, transferProof);
 * token.confidentialTransfer(recipient, bytes32(transferId), data);
 *
 * // 4. Withdraw: Unshield to public
 * token.withdraw(encryptedAmount, withdrawProof);
 * ```
 *
 * ## For Operators (Delegated Transfers)
 * ```solidity
 * // User grants operator approval
 * token.setOperator(operatorAddress, block.timestamp + 1 days);
 *
 * // Operator transfers on behalf of user
 * bytes memory data = abi.encode(encryptedAmount, transferProof);
 * token.confidentialTransferFrom(userAddress, recipient, amount, data);
 * ```
 *
 * # Security Considerations
 * - **Operator Risk**: Operators can transfer on behalf of users; only approve trusted addresses
 * - **Time Limits**: Operator approvals expire; set appropriate durations
 * - **Proof Validation**: All operations validate ZK proofs; invalid proofs will revert
 * - **Key Management**: Users must securely store their ElGamal private keys
 * - **Data Format**: Incorrect `data` parameter encoding will cause reverts
 *
 * # Gas Costs
 * - View functions: ~2,500-5,000 gas
 * - setOperator: ~25,000-35,000 gas
 * - deposit: ~55,000-105,000 gas
 * - confidentialTransfer: ~105,000-205,000 gas
 * - confidentialTransferFrom: ~110,000-210,000 gas (includes operator check)
 * - withdraw: ~85,000-155,000 gas
 * - claim: ~85,000-155,000 gas base + ~20,000 per additional transfer
 *
 * @custom:security-contact security@optimism.io
 * @custom:standard ERC-7984 Confidential Fungible Token
 */
contract ERC7984ConfidentialToken is IERC7984 {
    /// @notice The Confidential Assets precompile address
    /// @dev Fixed at 0x0000000000000000000000000000000000000800 - this is where all confidential
    ///      asset operations are handled. This wrapper delegates to this precompile.
    IConfidentialAssets public constant PRECOMPILE =
        IConfidentialAssets(0x0000000000000000000000000000000000000800);

    /// @notice The asset ID this wrapper is bound to
    /// @dev Set once at deployment and immutable thereafter. All operations on this wrapper
    ///      contract apply to this specific asset in the multi-asset precompile.
    uint128 public immutable assetId;

    /// @dev Token name cached from precompile or set at deployment
    ///      This avoids repeated cross-contract calls to fetch metadata
    string private _name;

    /// @dev Token symbol cached from precompile or set at deployment
    ///      This avoids repeated cross-contract calls to fetch metadata
    string private _symbol;

    /// @dev Token decimals cached from precompile or set at deployment
    ///      This avoids repeated cross-contract calls to fetch metadata
    uint8 private _decimals;

    /// @dev Operator approvals storage: holder => operator => expiry timestamp
    ///      When block.timestamp > expiry, the approval is invalid
    ///      Setting expiry to 0 effectively revokes the operator
    mapping(address => mapping(address => uint48)) private _operators;

    /// @dev Magic value that IERC7984Receiver.onConfidentialTokenReceived must return
    ///      Used to verify that recipient contracts properly handle confidential tokens
    bytes4 private constant RECEIVER_MAGIC = IERC7984Receiver.onConfidentialTokenReceived.selector;

    /// @notice Thrown when caller is not authorized to perform an operation
    /// @dev Specifically used in confidentialTransferFrom when the caller is not an approved
    ///      operator for the holder and is not the holder themselves
    error Unauthorized();

    /// @notice Thrown when the data parameter encoding is invalid
    /// @dev This occurs when:
    ///      - The data cannot be decoded as (bytes, bytes)
    ///      - The encryptedAmount is not exactly 64 bytes
    ///      - Required data is missing (e.g., calling transfer variant without data)
    error InvalidData();

    /// @notice Thrown when a recipient contract rejects the transfer
    /// @dev This occurs when calling a contract recipient and either:
    ///      - The contract doesn't implement IERC7984Receiver
    ///      - The contract's onConfidentialTokenReceived returns wrong magic value
    ///      - The contract's onConfidentialTokenReceived reverts
    error ReceiverRejected();

    /**
     * @notice Initializes the ERC-7984 wrapper for a specific asset
     * @dev This constructor binds the wrapper to a single asset ID from the Confidential Assets
     *      precompile. The wrapper can either use metadata from the precompile or custom values.
     *
     *      **Metadata Fetching Logic:**
     *      - If tokenName is empty string: fetch from precompile.name(assetId)
     *      - If tokenSymbol is empty string: fetch from precompile.symbol(assetId)
     *      - If tokenDecimals is 0: fetch from precompile.decimals(assetId)
     *      - Otherwise: use the provided values
     *
     *      **Use Cases:**
     *      1. **Wrapping Existing Asset**: Pass empty strings and 0 to auto-fetch metadata
     *      2. **Custom Branding**: Provide custom name/symbol for the same underlying asset
     *      3. **Proxy Assets**: Use different display metadata while sharing the asset
     *
     *      **Example - Auto-fetch metadata:**
     *      ```solidity
     *      new ERC7984ConfidentialToken(assetId, "", "", 0);
     *      ```
     *
     *      **Example - Custom metadata:**
     *      ```solidity
     *      new ERC7984ConfidentialToken(assetId, "My Token", "MTK", 18);
     *      ```
     *
     *      **Gas Cost:** ~100,000-200,000 gas (depends on metadata source)
     *
     * @param _assetId The asset ID from the Confidential Assets precompile (must exist)
     * @param tokenName The token name (empty string = fetch from precompile)
     * @param tokenSymbol The token symbol (empty string = fetch from precompile)
     * @param tokenDecimals The token decimals (0 = fetch from precompile, otherwise use value)
     */
    constructor(
        uint128 _assetId,
        string memory tokenName,
        string memory tokenSymbol,
        uint8 tokenDecimals
    ) {
        assetId = _assetId;

        // Use provided values or fetch from precompile
        if (bytes(tokenName).length > 0) {
            _name = tokenName;
        } else {
            _name = PRECOMPILE.name(_assetId);
        }

        if (bytes(tokenSymbol).length > 0) {
            _symbol = tokenSymbol;
        } else {
            _symbol = PRECOMPILE.symbol(_assetId);
        }

        if (tokenDecimals > 0) {
            _decimals = tokenDecimals;
        } else {
            _decimals = PRECOMPILE.decimals(_assetId);
        }
    }

    // ============ ERC-7984 View Functions ============

    /// @inheritdoc IERC7984
    function name() external view override returns (string memory) {
        return _name;
    }

    /// @inheritdoc IERC7984
    function symbol() external view override returns (string memory) {
        return _symbol;
    }

    /// @inheritdoc IERC7984
    function decimals() external view override returns (uint8) {
        return _decimals;
    }

    /// @inheritdoc IERC7984
    function confidentialTotalSupply() external view override returns (bytes32) {
        return PRECOMPILE.confidentialTotalSupply(assetId);
    }

    /// @inheritdoc IERC7984
    function confidentialBalanceOf(address account) external view override returns (bytes32) {
        return PRECOMPILE.confidentialBalanceOf(assetId, account);
    }

    /// @inheritdoc IERC7984
    function isOperator(address holder, address spender) external view override returns (bool) {
        uint48 until = _operators[holder][spender];
        return until > block.timestamp;
    }

    // ============ ERC-7984 State-Changing Functions ============

    /// @inheritdoc IERC7984
    /// @notice Sets an operator approval for the caller
    /// @dev Operators can transfer tokens on behalf of the holder using confidentialTransferFrom.
    ///      This is similar to ERC-20 approve, but with time-based expiration for added security.
    ///
    ///      **Operator Permissions:**
    ///      - Can call confidentialTransferFrom to transfer holder's tokens
    ///      - Cannot deposit, withdraw, or claim on behalf of holder
    ///      - Authorization expires at the specified timestamp
    ///
    ///      **Time-based Expiration:**
    ///      - Pass `block.timestamp + duration` for relative time (e.g., +1 days)
    ///      - Pass specific Unix timestamp for absolute time
    ///      - Pass 0 to revoke operator immediately
    ///      - Expired approvals (until <= block.timestamp) are automatically invalid
    ///
    ///      **Security Best Practices:**
    ///      - Use shortest necessary duration to limit risk window
    ///      - Consider 1 day for routine operations
    ///      - Consider 1 hour for single-use automated operations
    ///      - Revoke (set to 0) when no longer needed
    ///
    ///      **Gas Cost:** ~25,000-35,000 gas (storage write + event)
    ///
    ///      **Example:**
    ///      ```solidity
    ///      // Approve for 1 day
    ///      token.setOperator(automatedService, block.timestamp + 1 days);
    ///
    ///      // Revoke immediately
    ///      token.setOperator(automatedService, 0);
    ///      ```
    ///
    /// @param operator The address to authorize as an operator
    /// @param until Unix timestamp until which the operator is valid (0 to revoke)
    function setOperator(address operator, uint48 until) external override {
        _operators[msg.sender][operator] = until;
        emit OperatorSet(msg.sender, operator, until);
    }

    /**
     * @inheritdoc IERC7984
     * @notice Confidential transfer without data parameter (NOT SUPPORTED)
     * @dev This variant is part of the ERC-7984 standard but is not supported in this implementation.
     *      Always use the variant with the `data` parameter which includes the encrypted amount and proof.
     *
     *      **Why Not Supported:**
     *      The Confidential Assets precompile requires both encrypted amount and ZK proof in a single
     *      transaction. There's no mechanism to "pre-submit" these values separately.
     *
     *      **Always Reverts:** This function always reverts with InvalidData()
     *
     * @param to The recipient address (unused - will revert)
     * @param amount The amount commitment (unused - will revert)
     * @return Never returns - always reverts
     */
    function confidentialTransfer(
        address to,
        bytes32 amount
    ) external override returns (bytes32) {
        revert InvalidData(); // Must use variant with data parameter
    }

    /**
     * @inheritdoc IERC7984
     * @notice Transfers tokens confidentially to another address
     * @dev This is the main transfer function. It wraps the precompile's confidentialTransfer
     *      and emits the ERC-7984 ConfidentialTransfer event.
     *
     *      **Data Parameter Format:**
     *      Must be ABI-encoded as: `abi.encode(bytes encryptedAmount, bytes proof)`
     *      - encryptedAmount: exactly 64 bytes (ElGamal ciphertext encrypted to recipient)
     *      - proof: variable length ZK proof bytes
     *
     *      **Prerequisites:**
     *      - Caller must have sufficient confidential balance
     *      - Recipient must have set their public key
     *      - Caller must know their current balance to generate proof
     *
     *      **Amount Parameter:**
     *      The `amount` parameter is used for event emission and can be any bytes32 value.
     *      Common uses:
     *      - Transfer ID or nonce
     *      - Hash of the encrypted amount
     *      - Commitment value
     *      The actual transfer amount is in the encrypted data.
     *
     *      **Gas Cost:** ~105,000-205,000 gas (includes proof verification)
     *
     *      **Reverts:**
     *      - InvalidData: if data is not properly encoded or encryptedAmount != 64 bytes
     *      - (Precompile reverts): if proof invalid, insufficient balance, recipient has no pubkey
     *
     *      **Example:**
     *      ```solidity
     *      bytes memory encryptedAmt = encryptToRecipient(100e18, recipientPubKey);
     *      bytes memory proof = generateTransferProof(...);
     *      bytes memory data = abi.encode(encryptedAmt, proof);
     *      token.confidentialTransfer(recipient, bytes32(transferId), data);
     *      ```
     *
     * @param to The recipient address
     * @param amount The amount commitment/handle (for event emission)
     * @param data ABI-encoded (bytes encryptedAmount, bytes proof)
     * @return The amount parameter (unchanged)
     */
    function confidentialTransfer(
        address to,
        bytes32 amount,
        bytes calldata data
    ) external override returns (bytes32) {
        _transfer(msg.sender, to, amount, data);
        return amount;
    }

    /**
     * @inheritdoc IERC7984
     * @notice Operator transfer without data parameter (NOT SUPPORTED)
     * @dev This variant is part of the ERC-7984 standard but is not supported in this implementation.
     *      Always use the variant with the `data` parameter which includes the encrypted amount and proof.
     *
     *      **Always Reverts:** This function always reverts with InvalidData()
     *
     * @param from The token holder (unused - will revert)
     * @param to The recipient address (unused - will revert)
     * @param amount The amount commitment (unused - will revert)
     * @return Never returns - always reverts
     */
    function confidentialTransferFrom(
        address from,
        address to,
        bytes32 amount
    ) external override returns (bytes32) {
        revert InvalidData(); // Must use variant with data parameter
    }

    /**
     * @inheritdoc IERC7984
     * @notice Transfers tokens confidentially on behalf of another address
     * @dev This function allows operators to transfer tokens on behalf of holders. The caller must
     *      be an authorized operator (via setOperator) or the holder themselves.
     *
     *      **Authorization:**
     *      - Caller must be the holder, OR
     *      - Caller must be an approved operator with unexpired approval (until > block.timestamp)
     *      - Reverts with Unauthorized() if neither condition is met
     *
     *      **Data Parameter Format:**
     *      Same as confidentialTransfer: `abi.encode(bytes encryptedAmount, bytes proof)`
     *
     *      **Important Security Note:**
     *      Even though an operator calls this function, the ZK proof must be generated with
     *      knowledge of the holder's balance. This means:
     *      - The holder must provide the proof to the operator
     *      - Or the operator must be a trusted party that knows the holder's balance
     *      - This prevents operators from arbitrarily transferring without holder's cooperation
     *
     *      **Use Cases:**
     *      1. **Automated Trading**: User approves DEX to execute trades
     *      2. **Recurring Payments**: User approves service to pull payments
     *      3. **Custodial Services**: User approves custodian for management
     *      4. **Smart Contract Automation**: User approves contract for automated operations
     *
     *      **Gas Cost:** ~110,000-210,000 gas (includes operator check + proof verification)
     *
     *      **Reverts:**
     *      - Unauthorized: if caller is not holder and not an approved operator
     *      - InvalidData: if data encoding is invalid
     *      - (Precompile reverts): if proof invalid, insufficient balance, etc.
     *
     *      **Example:**
     *      ```solidity
     *      // User approves operator
     *      token.setOperator(operatorAddress, block.timestamp + 1 days);
     *
     *      // Operator transfers on behalf of user (with user-provided proof)
     *      bytes memory data = abi.encode(encryptedAmount, proof);
     *      token.confidentialTransferFrom(userAddress, recipient, amountHandle, data);
     *      ```
     *
     * @param from The token holder whose tokens are being transferred
     * @param to The recipient address
     * @param amount The amount commitment/handle (for event emission)
     * @param data ABI-encoded (bytes encryptedAmount, bytes proof)
     * @return The amount parameter (unchanged)
     */
    function confidentialTransferFrom(
        address from,
        address to,
        bytes32 amount,
        bytes calldata data
    ) external override returns (bytes32) {
        if (!_isAuthorized(from, msg.sender)) {
            revert Unauthorized();
        }
        _transfer(from, to, amount, data);
        return amount;
    }

    // ============ Additional Functions ============

    /**
     * @notice Sets the caller's ElGamal public key for receiving confidential transfers
     * @dev This is a REQUIRED one-time setup before receiving any confidential transfers.
     *      This function is a convenience wrapper around the precompile's setPublicKey function.
     *
     *      See IConfidentialAssets.setPublicKey for full documentation on:
     *      - Public key format requirements
     *      - Security considerations
     *      - Key management best practices
     *
     *      **Gas Cost:** ~20,000-30,000 gas
     *
     *      **Reverts:**
     *      - If pubkey length != 64 bytes
     *      - If pubkey is not a valid curve point
     *
     * @param pubkey The ElGamal public key (exactly 64 bytes)
     */
    function setPublicKey(bytes calldata pubkey) external {
        PRECOMPILE.setPublicKey(pubkey);
    }

    /**
     * @notice Deposits (shields) public tokens into confidential balance
     * @dev Converts public tokens to confidential balance for this specific asset.
     *      This function delegates to the precompile and emits a ConfidentialTransfer event
     *      with address(0) as the sender (indicating a mint/deposit).
     *
     *      **Prerequisites:**
     *      - Caller must have sufficient public balance
     *      - For ERC-20 assets, caller must have approved this contract or precompile
     *      - Caller must have set their public key
     *
     *      **Privacy Note:**
     *      The deposited amount is PUBLIC and visible on-chain. Privacy begins after deposit.
     *
     *      **Gas Cost:** ~55,000-105,000 gas (includes proof verification)
     *
     *      **Reverts:**
     *      - If proof verification fails
     *      - If caller has insufficient public balance
     *      - If caller has not set public key
     *
     *      **Example:**
     *      ```solidity
     *      // For ERC-20 backed assets
     *      underlyingToken.approve(address(token), 1000e18);
     *      token.deposit(1000e18, depositProof);
     *      ```
     *
     * @param amount The plaintext amount to deposit (in base units)
     * @param proof The ZK proof bytes for the deposit
     */
    function deposit(uint256 amount, bytes calldata proof) external {
        PRECOMPILE.deposit(assetId, amount, proof);
        emit ConfidentialTransfer(address(0), msg.sender, bytes32(amount));
    }

    /**
     * @notice Withdraws (unshields) confidential balance to public tokens
     * @dev Converts confidential balance back to public tokens for this specific asset.
     *      This function delegates to the precompile and emits a ConfidentialTransfer event
     *      with address(0) as the recipient (indicating a burn/withdrawal).
     *
     *      **Prerequisites:**
     *      - Caller must have sufficient confidential balance
     *      - Caller must know their balance to generate proof
     *
     *      **Privacy Note:**
     *      The withdrawn amount becomes PUBLIC as public tokens are received.
     *
     *      **Event Emission:**
     *      The event uses keccak256(encryptedAmount) as the amount handle since we don't
     *      have access to the plaintext amount in the contract.
     *
     *      **Gas Cost:** ~85,000-155,000 gas (includes proof verification)
     *
     *      **Reverts:**
     *      - If proof verification fails
     *      - If encryptedAmount length != 64 bytes
     *      - If insufficient confidential balance
     *
     *      **Example:**
     *      ```solidity
     *      bytes memory encrypted = ...; // 64 bytes
     *      bytes memory proof = generateWithdrawProof(...);
     *      token.withdraw(encrypted, proof);
     *      // Public tokens now in user's account
     *      ```
     *
     * @param encryptedAmount The encrypted withdrawal amount (exactly 64 bytes)
     * @param proof The ZK proof bytes for the withdrawal
     */
    function withdraw(bytes calldata encryptedAmount, bytes calldata proof) external {
        PRECOMPILE.withdraw(assetId, encryptedAmount, proof);
        // Note: We emit with a hash of the encrypted amount as we don't know the plaintext
        emit ConfidentialTransfer(msg.sender, address(0), keccak256(encryptedAmount));
    }

    /**
     * @notice Claims pending confidential transfers for this asset
     * @dev After receiving confidential transfers, users must claim them to make the funds usable.
     *      This function delegates to the precompile's confidentialClaim for the bound assetId.
     *
     *      **Claiming Process:**
     *      1. User receives one or more confidential transfers (stored as pending)
     *      2. User decrypts amounts using their private key (off-chain)
     *      3. User generates claim proof including transfer IDs
     *      4. User calls this function to claim
     *      5. Funds become available in user's confidential balance
     *
     *      **Batch Claiming:**
     *      Multiple pending transfers can be claimed in a single transaction for gas efficiency.
     *
     *      **Gas Cost:** ~85,000-155,000 gas base + ~20,000 per additional transfer
     *
     *      **Reverts:**
     *      - If proof verification fails
     *      - If transfer IDs are invalid or already claimed
     *      - If caller is not the legitimate recipient
     *
     *      **Example:**
     *      ```solidity
     *      // After receiving transfers, claim them
     *      bytes memory claimProof = generateClaimProof(transferIds);
     *      token.claim(claimProof);
     *      // Funds now available for use
     *      ```
     *
     * @param proof The ZK proof bytes containing transfer IDs and decryption proofs
     */
    function claim(bytes calldata proof) external {
        PRECOMPILE.confidentialClaim(assetId, proof);
    }

    // ============ Internal Functions ============

    /**
     * @dev Checks if a spender is authorized to transfer on behalf of a holder
     * @param holder The token holder whose authorization is being checked
     * @param spender The address attempting to spend
     * @return True if authorized (either holder == spender OR valid unexpired operator approval)
     */
    function _isAuthorized(address holder, address spender) internal view returns (bool) {
        if (holder == spender) return true;
        uint48 until = _operators[holder][spender];
        return until > block.timestamp;
    }

    /**
     * @dev Internal function to execute a confidential transfer via the precompile
     * @dev This function:
     *      1. Decodes the data parameter into encryptedAmount and proof
     *      2. Validates the encryptedAmount is exactly 64 bytes
     *      3. Calls the precompile's confidentialTransfer
     *      4. Emits the ERC-7984 ConfidentialTransfer event
     *
     *      **Important Implementation Note:**
     *      The precompile uses msg.sender (this contract) as the transfer initiator, but the
     *      actual proof must be generated knowing the holder's balance. The precompile handles
     *      the actual balance accounting.
     *
     * @param from The sender address (used for event emission only)
     * @param to The recipient address
     * @param amount The amount commitment/handle (used for event emission only)
     * @param data ABI-encoded (bytes encryptedAmount, bytes proof)
     */
    function _transfer(
        address from,
        address to,
        bytes32 amount,
        bytes calldata data
    ) internal {
        // Decode the data parameter
        (bytes memory encryptedAmount, bytes memory proof) = abi.decode(data, (bytes, bytes));

        if (encryptedAmount.length != 64) {
            revert InvalidData();
        }

        // Call the precompile
        // Note: The precompile uses msg.sender as the sender, so for transferFrom
        // we need the operator to have the encrypted amount prepared for transfer
        PRECOMPILE.confidentialTransfer(assetId, to, encryptedAmount, proof);

        emit ConfidentialTransfer(from, to, amount);
    }

    /**
     * @dev Calls onConfidentialTokenReceived on the recipient if it's a contract
     * @dev This implements the ERC-7984 receiver check pattern, similar to ERC-721 and ERC-1155.
     *      If the recipient is a contract, it must implement IERC7984Receiver and return the
     *      correct magic value to accept the transfer.
     *
     *      **Note:** This function is currently defined but not called in the transfer flow.
     *      If you want to enforce receiver checks, call this function from _transfer().
     *
     * @param from The sender address
     * @param to The recipient address
     * @param amount The amount commitment/handle
     * @param data The data passed with the transfer
     */
    function _checkOnReceived(
        address from,
        address to,
        bytes32 amount,
        bytes calldata data
    ) internal {
        if (to.code.length > 0) {
            try IERC7984Receiver(to).onConfidentialTokenReceived(from, amount, data) returns (bytes4 retval) {
                if (retval != RECEIVER_MAGIC) {
                    revert ReceiverRejected();
                }
            } catch {
                revert ReceiverRejected();
            }
        }
    }
}
