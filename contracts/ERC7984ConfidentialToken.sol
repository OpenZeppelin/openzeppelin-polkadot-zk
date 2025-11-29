// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC7984, IERC7984Receiver} from "./interfaces/IERC7984.sol";
import {IConfidentialAssets} from "./interfaces/IConfidentialAssets.sol";

/**
 * @title ERC7984ConfidentialToken
 * @dev ERC-7984 compliant wrapper around the Confidential Assets precompile.
 *
 * This contract wraps a specific asset from the multi-asset Confidential Assets
 * precompile and exposes it as an ERC-7984 compliant confidential token.
 *
 * The wrapper:
 * - Binds to a specific asset ID at deployment
 * - Translates ERC-7984 calls to precompile calls
 * - Manages operator approvals (time-limited delegations)
 * - Handles the `data` parameter as ZK proofs for transfers
 *
 * Note: The ERC-7984 `bytes32 amount` parameter is interpreted as a commitment/handle.
 * The actual encrypted amount and proof must be passed via the `data` parameter.
 */
contract ERC7984ConfidentialToken is IERC7984 {
    /// @dev The Confidential Assets precompile address
    IConfidentialAssets public constant PRECOMPILE =
        IConfidentialAssets(0x0000000000000000000000000000000000000800);

    /// @dev The asset ID this wrapper is bound to
    uint128 public immutable assetId;

    /// @dev Token name (cached from precompile or set at deployment)
    string private _name;

    /// @dev Token symbol (cached from precompile or set at deployment)
    string private _symbol;

    /// @dev Token decimals (cached from precompile or set at deployment)
    uint8 private _decimals;

    /// @dev Operator approvals: holder => operator => expiry timestamp
    mapping(address => mapping(address => uint48)) private _operators;

    /// @dev Magic value returned by IERC7984Receiver.onConfidentialTokenReceived
    bytes4 private constant RECEIVER_MAGIC = IERC7984Receiver.onConfidentialTokenReceived.selector;

    /// @dev Error thrown when caller is not authorized
    error Unauthorized();

    /// @dev Error thrown when data parameter is invalid
    error InvalidData();

    /// @dev Error thrown when receiver rejects the transfer
    error ReceiverRejected();

    /**
     * @dev Initializes the wrapper for a specific asset.
     * @param _assetId The asset ID from the Confidential Assets precompile
     * @param tokenName The token name (use empty string to fetch from precompile)
     * @param tokenSymbol The token symbol (use empty string to fetch from precompile)
     * @param tokenDecimals The token decimals (use 0 to fetch from precompile)
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
    function setOperator(address operator, uint48 until) external override {
        _operators[msg.sender][operator] = until;
        emit OperatorSet(msg.sender, operator, until);
    }

    /**
     * @inheritdoc IERC7984
     * @dev This variant requires the caller to have previously submitted the
     * encrypted amount and proof via a separate transaction or mechanism.
     * For full functionality, use the variant with `data` parameter.
     */
    function confidentialTransfer(
        address to,
        bytes32 amount
    ) external override returns (bytes32) {
        revert InvalidData(); // Must use variant with data parameter
    }

    /**
     * @inheritdoc IERC7984
     * @dev The `data` parameter must be ABI-encoded as:
     * (bytes encryptedAmount, bytes proof)
     * where encryptedAmount is exactly 64 bytes.
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
     * @dev This variant requires the caller to have previously submitted the
     * encrypted amount and proof via a separate transaction or mechanism.
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
     * @dev The caller must be an authorized operator for `from`.
     * The `data` parameter must be ABI-encoded as:
     * (bytes encryptedAmount, bytes proof)
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
     * @dev Sets the public key for the caller to receive confidential transfers.
     * This is required before receiving any confidential transfers.
     * @param pubkey The ElGamal public key (64 bytes)
     */
    function setPublicKey(bytes calldata pubkey) external {
        PRECOMPILE.setPublicKey(pubkey);
    }

    /**
     * @dev Deposits public tokens into confidential balance (shielding).
     * @param amount The amount to deposit
     * @param proof The ZK proof for the deposit
     */
    function deposit(uint256 amount, bytes calldata proof) external {
        PRECOMPILE.deposit(assetId, amount, proof);
        emit ConfidentialTransfer(address(0), msg.sender, bytes32(amount));
    }

    /**
     * @dev Withdraws confidential balance to public tokens (unshielding).
     * @param encryptedAmount The encrypted amount (64 bytes)
     * @param proof The ZK proof for the withdrawal
     */
    function withdraw(bytes calldata encryptedAmount, bytes calldata proof) external {
        PRECOMPILE.withdraw(assetId, encryptedAmount, proof);
        // Note: We emit with a hash of the encrypted amount as we don't know the plaintext
        emit ConfidentialTransfer(msg.sender, address(0), keccak256(encryptedAmount));
    }

    /**
     * @dev Claims pending confidential deposits.
     * @param proof The ZK proof containing transfer IDs to claim
     */
    function claim(bytes calldata proof) external {
        PRECOMPILE.confidentialClaim(assetId, proof);
    }

    // ============ Internal Functions ============

    /**
     * @dev Checks if spender is authorized to transfer on behalf of holder.
     */
    function _isAuthorized(address holder, address spender) internal view returns (bool) {
        if (holder == spender) return true;
        uint48 until = _operators[holder][spender];
        return until > block.timestamp;
    }

    /**
     * @dev Executes a confidential transfer via the precompile.
     * @param from The sender (for event emission, actual sender is msg.sender to precompile)
     * @param to The recipient
     * @param amount The amount commitment (for event emission)
     * @param data ABI-encoded (encryptedAmount, proof)
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
     * @dev Calls onConfidentialTokenReceived on the recipient if it's a contract.
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
