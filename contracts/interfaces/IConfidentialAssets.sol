// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title IConfidentialAssets
/// @author OpenZeppelin
/// @notice Interface for the Confidential Assets precompile
/// @dev This precompile provides multi-asset confidential token functionality with ZK-proof based privacy.
///
/// # Overview
/// The Confidential Assets precompile enables privacy-preserving token operations using zero-knowledge proofs
/// and homomorphic encryption (ElGamal). It supports:
/// - Multiple assets managed by a single precompile instance
/// - Shielding: Converting public tokens to confidential balances (deposit)
/// - Unshielding: Converting confidential balances to public tokens (withdraw)
/// - Confidential transfers: Moving tokens between users without revealing amounts
/// - Confidential claims: Claiming pending transfers from other users
///
/// # Architecture
/// - **Precompile Address**: 0x0000000000000000000000000000000000000800 (fixed deployment)
/// - **Asset IDs**: Each asset is identified by a uint128 ID
/// - **Balance Commitments**: Balances are stored as 32-byte Pedersen commitments
/// - **Encrypted Amounts**: Transfer amounts are ElGamal encrypted (64 bytes)
/// - **ZK Proofs**: All operations require validity proofs to prevent double-spending
///
/// # Security Model
/// - **Privacy**: Balances and amounts are encrypted; only commitments are stored on-chain
/// - **Validity**: ZK proofs ensure operations are mathematically valid without revealing amounts
/// - **Public Keys**: Each user must set an ElGamal public key to receive confidential transfers
/// - **No Frontrunning**: Encrypted amounts prevent MEV extraction based on transfer values
///
/// # Integration Steps
/// 1. Deploy or identify the asset ID to work with
/// 2. Users call `setPublicKey()` with their ElGamal public key (64 bytes)
/// 3. Users can `deposit()` public tokens to get confidential balance
/// 4. Users can perform `confidentialTransfer()` to send encrypted amounts
/// 5. Recipients can `confidentialClaim()` to claim pending transfers
/// 6. Users can `withdraw()` to convert confidential balance back to public tokens
///
/// # Gas Considerations
/// - View functions (balanceOf, totalSupply): ~2,000-5,000 gas
/// - setPublicKey: ~20,000-30,000 gas (one-time setup)
/// - deposit: ~50,000-100,000 gas (includes proof verification)
/// - confidentialTransfer: ~100,000-200,000 gas (proof verification + encryption)
/// - confidentialClaim: ~80,000-150,000 gas (depends on number of claims)
/// - withdraw: ~80,000-150,000 gas (proof verification + balance update)
///
/// # Example Usage
/// ```solidity
/// IConfidentialAssets precompile = IConfidentialAssets(0x0000000000000000000000000000000000000800);
///
/// // Setup: Set public key (one-time)
/// bytes memory myPublicKey = ...; // 64 bytes from ElGamal key generation
/// precompile.setPublicKey(myPublicKey);
///
/// // Shield tokens: Deposit 100 tokens (assumes approval given)
/// bytes memory depositProof = ...; // Generated off-chain
/// precompile.deposit(assetId, 100, depositProof);
///
/// // Transfer confidentially
/// bytes memory encryptedAmount = ...; // Encrypted amount (64 bytes)
/// bytes memory transferProof = ...; // ZK proof
/// precompile.confidentialTransfer(assetId, recipient, encryptedAmount, transferProof);
///
/// // Unshield: Withdraw back to public balance
/// bytes memory withdrawProof = ...; // Includes plaintext amount
/// precompile.withdraw(assetId, encryptedAmount, withdrawProof);
/// ```
///
/// @custom:deployment-address 0x0000000000000000000000000000000000000800
/// @custom:security-contact security@openzeppelin.com
///
/// NOTE: The @custom:selector annotations are verified by Rust tests in
/// precompiles/confidential-assets-evm/src/tests.rs to ensure the Solidity
/// interface matches the Rust precompile implementation.
interface IConfidentialAssets {
    /// @notice Returns the encrypted balance commitment for an account
    /// @dev The returned value is a Pedersen commitment to the account's balance. It does not reveal
    ///      the actual balance amount but can be used in ZK proofs to verify operations.
    ///
    ///      **Technical Details:**
    ///      - The commitment is computed as: C = vG + rH where v is balance, G and H are curve points
    ///      - Multiple commitments can be homomorphically added/subtracted
    ///      - The commitment is deterministic for a given balance and randomness
    ///
    ///      **Usage:**
    ///      - Use this to get a user's balance commitment for proof generation
    ///      - Compare commitments to verify balance hasn't changed unexpectedly
    ///      - Display to users as a "fingerprint" of their private balance
    ///
    ///      **Gas Cost:** ~2,500 gas (simple storage read)
    ///
    /// @param asset The asset ID (uint128) identifying which token to query
    /// @param who The account address whose balance to query
    /// @return commitment The 32-byte Pedersen commitment representing the encrypted balance
    ///
    /// @custom:selector cd40095b
    /// @custom:security This function is safe to call publicly; it reveals no sensitive information
    function confidentialBalanceOf(uint128 asset, address who) external view returns (bytes32 commitment);

    /// @notice Returns the encrypted total supply commitment for an asset
    /// @dev The total supply commitment represents the sum of all confidential balances for this asset.
    ///      Like individual balances, this is a Pedersen commitment that hides the actual value.
    ///
    ///      **Properties:**
    ///      - Updated automatically when deposits, withdrawals, or transfers occur
    ///      - Sum of all individual balance commitments should equal this value
    ///      - Can be used to verify the system's integrity
    ///
    ///      **Gas Cost:** ~2,500 gas (simple storage read)
    ///
    /// @param asset The asset ID (uint128) identifying which token to query
    /// @return commitment The 32-byte Pedersen commitment representing the encrypted total supply
    ///
    /// @custom:selector efa18641
    /// @custom:security This function is safe to call publicly; it reveals no sensitive information
    function confidentialTotalSupply(uint128 asset) external view returns (bytes32 commitment);

    /// @notice Returns the human-readable name of an asset
    /// @dev Standard ERC-20 compatible name function, but takes an asset ID parameter since
    ///      this precompile manages multiple assets.
    ///
    ///      **Gas Cost:** ~3,000-10,000 gas (depends on string length)
    ///
    /// @param asset The asset ID (uint128) to query
    /// @return tokenName The human-readable name (e.g., "Confidential USD Coin")
    ///
    /// @custom:selector c624440a
    function name(uint128 asset) external view returns (string memory tokenName);

    /// @notice Returns the ticker symbol of an asset
    /// @dev Standard ERC-20 compatible symbol function, but takes an asset ID parameter.
    ///
    ///      **Gas Cost:** ~3,000-8,000 gas (depends on string length)
    ///
    /// @param asset The asset ID (uint128) to query
    /// @return tokenSymbol The ticker symbol (e.g., "cUSDC")
    ///
    /// @custom:selector 117f1264
    function symbol(uint128 asset) external view returns (string memory tokenSymbol);

    /// @notice Returns the number of decimals used for display purposes
    /// @dev Standard ERC-20 compatible decimals function. This is purely for UI display and does
    ///      not affect the internal representation of amounts.
    ///
    ///      **Example:** If decimals = 18, then 1e18 base units = 1.0 tokens in UIs
    ///
    ///      **Gas Cost:** ~2,500 gas (simple storage read)
    ///
    /// @param asset The asset ID (uint128) to query
    /// @return decimalPlaces The number of decimal places (typically 18 for most tokens)
    ///
    /// @custom:selector 09d2f9b4
    function decimals(uint128 asset) external view returns (uint8 decimalPlaces);

    /// @notice Sets the caller's ElGamal public key for receiving confidential transfers
    /// @dev This is a REQUIRED one-time setup before receiving any confidential transfers. The public key
    ///      is used to encrypt transfer amounts so only the recipient can decrypt them.
    ///
    ///      **Public Key Format:**
    ///      - Must be exactly 64 bytes
    ///      - Represents an ElGamal public key (elliptic curve point)
    ///      - Generated off-chain from a private key the user controls
    ///      - Format: 32 bytes X coordinate + 32 bytes Y coordinate
    ///
    ///      **Security Considerations:**
    ///      - The private key corresponding to this public key MUST be kept secret
    ///      - Loss of private key means inability to decrypt received amounts
    ///      - Public key can be updated, but old encrypted amounts won't be decryptable with new key
    ///      - Consider using deterministic key derivation (BIP-32/44) for key recovery
    ///
    ///      **Integration Flow:**
    ///      1. Generate ElGamal key pair off-chain (private key stays client-side)
    ///      2. Call this function with the public key (one-time)
    ///      3. Store private key securely (needed to decrypt received amounts)
    ///      4. Now able to receive confidential transfers
    ///
    ///      **Gas Cost:** ~20,000-30,000 gas (storage write + validation)
    ///
    ///      **Reverts:**
    ///      - If pubkey length != 64 bytes
    ///      - If pubkey is not a valid curve point
    ///
    /// @param pubkey The ElGamal public key (exactly 64 bytes: 32-byte X + 32-byte Y coordinate)
    ///
    /// @custom:selector a91d58b4
    /// @custom:security CRITICAL - Keep the corresponding private key secure and backed up
    function setPublicKey(bytes calldata pubkey) external;

    /// @notice Deposits (shields) public assets into confidential balance
    /// @dev Converts public tokens into confidential balance. This operation:
    ///      1. Burns/locks public tokens from caller's account
    ///      2. Increases caller's confidential balance commitment
    ///      3. Updates the confidential total supply commitment
    ///
    ///      **Prerequisites:**
    ///      - Caller must have sufficient public balance of the asset
    ///      - If asset is ERC-20, caller must have approved precompile as spender
    ///      - Caller must have set their public key (via setPublicKey)
    ///
    ///      **ZK Proof:**
    ///      The proof must demonstrate:
    ///      - Knowledge of the plaintext amount being deposited
    ///      - Correct formation of the balance commitment
    ///      - The amount matches what's being transferred from public balance
    ///
    ///      **Privacy Note:**
    ///      The `amount` parameter is PUBLIC and visible on-chain. Privacy begins after deposit.
    ///      The resulting confidential balance is private, but the deposit amount itself is not.
    ///
    ///      **Gas Cost:** ~50,000-100,000 gas (includes proof verification and balance updates)
    ///
    ///      **Reverts:**
    ///      - If caller has insufficient public balance
    ///      - If proof verification fails
    ///      - If caller has not set public key
    ///      - If amount is 0 or would cause overflow
    ///
    ///      **Example:**
    ///      ```solidity
    ///      // 1. Generate deposit proof off-chain
    ///      // 2. Approve precompile (if ERC-20)
    ///      token.approve(precompileAddress, amount);
    ///      // 3. Deposit
    ///      precompile.deposit(assetId, 1000e18, depositProof);
    ///      ```
    ///
    /// @param asset The asset ID (uint128) to deposit
    /// @param amount The plaintext amount to deposit (in base units, e.g., wei for 18 decimals)
    /// @param proof The ZK proof bytes proving the deposit is valid
    ///
    /// @custom:selector 94679bd1
    /// @custom:security The amount is public during deposit; confidentiality applies to the resulting balance
    function deposit(uint128 asset, uint256 amount, bytes calldata proof) external;

    /// @notice Withdraws (unshields) confidential balance to public assets
    /// @dev Converts confidential balance back to public tokens. This operation:
    ///      1. Decreases caller's confidential balance commitment
    ///      2. Mints/unlocks public tokens to caller's account
    ///      3. Updates the confidential total supply commitment
    ///
    ///      **Prerequisites:**
    ///      - Caller must have sufficient confidential balance
    ///      - Caller must know their confidential balance (to generate proof)
    ///
    ///      **Encrypted Amount:**
    ///      - Must be exactly 64 bytes (ElGamal ciphertext)
    ///      - Represents the encrypted withdrawal amount
    ///      - Only decryptable by the user who has the private key
    ///      - Observers cannot determine the withdrawal amount from this value
    ///
    ///      **ZK Proof:**
    ///      The proof must demonstrate:
    ///      - Caller has sufficient confidential balance for the withdrawal
    ///      - The encrypted amount is correctly formed
    ///      - The plaintext amount matches what's being withdrawn
    ///      - The new balance commitment is correctly computed
    ///
    ///      **Privacy Note:**
    ///      The withdrawal amount is revealed as PUBLIC tokens are received. While the
    ///      encryptedAmount parameter is opaque, the resulting public balance change is visible.
    ///
    ///      **Gas Cost:** ~80,000-150,000 gas (proof verification + balance updates)
    ///
    ///      **Reverts:**
    ///      - If proof verification fails (including insufficient balance)
    ///      - If encryptedAmount length != 64 bytes
    ///      - If amount is 0 or would cause overflow
    ///      - If decrypted amount doesn't match proof
    ///
    ///      **Example:**
    ///      ```solidity
    ///      // 1. User decrypts their balance off-chain using private key
    ///      // 2. Generate withdrawal proof for desired amount
    ///      // 3. Withdraw
    ///      bytes memory encrypted = ...; // 64 bytes
    ///      bytes memory proof = ...; // ZK proof
    ///      precompile.withdraw(assetId, encrypted, proof);
    ///      // 4. Public tokens now in user's account
    ///      ```
    ///
    /// @param asset The asset ID (uint128) to withdraw from
    /// @param encryptedAmount The encrypted withdrawal amount (exactly 64 bytes: ElGamal ciphertext)
    /// @param proof The ZK proof bytes proving the withdrawal is valid
    ///
    /// @custom:selector f1f9153b
    /// @custom:security The withdrawn amount becomes public; ensure this is acceptable for your use case
    function withdraw(uint128 asset, bytes calldata encryptedAmount, bytes calldata proof) external;

    /// @notice Performs a confidential transfer to another user
    /// @dev Transfers confidential balance from caller to recipient. This is the core privacy-preserving
    ///      operation of the system. Neither the amount nor the updated balances are revealed.
    ///
    ///      **Operation Flow:**
    ///      1. Decreases sender's confidential balance commitment
    ///      2. Increases recipient's confidential balance commitment
    ///      3. Creates a pending transfer for recipient to claim
    ///      4. Observers see only encrypted data, no amounts
    ///
    ///      **Prerequisites:**
    ///      - Sender must have sufficient confidential balance
    ///      - Recipient must have set their public key (to encrypt the amount)
    ///      - Sender must know their current balance (for proof generation)
    ///
    ///      **Encrypted Amount:**
    ///      - Must be exactly 64 bytes (ElGamal ciphertext)
    ///      - Encrypted using recipient's public key
    ///      - Only recipient can decrypt using their private key
    ///      - Contains the transfer amount in encrypted form
    ///
    ///      **ZK Proof:**
    ///      The proof must demonstrate:
    ///      - Sender has sufficient balance for the transfer
    ///      - The encrypted amount is correctly formed
    ///      - The sender knows the plaintext amount
    ///      - The new balance commitments are correctly computed
    ///      - No negative balances or overflows occur
    ///
    ///      **Privacy Guarantees:**
    ///      - Transfer amount is fully hidden from observers
    ///      - Sender and recipient balances remain confidential
    ///      - Only sender and recipient know the transfer amount
    ///      - Transaction graph (who sent to whom) is public, but amounts are private
    ///
    ///      **Claiming:**
    ///      The recipient must call `confidentialClaim()` to actually claim the transfer
    ///      into their usable balance. Until claimed, the transfer is in a "pending" state.
    ///
    ///      **Gas Cost:** ~100,000-200,000 gas (proof verification + balance updates + encryption)
    ///
    ///      **Reverts:**
    ///      - If proof verification fails (including insufficient balance)
    ///      - If encryptedAmount length != 64 bytes
    ///      - If recipient has not set their public key
    ///      - If amount is 0 or would cause overflow
    ///      - If to address is zero address
    ///
    ///      **Example:**
    ///      ```solidity
    ///      // 1. Get recipient's public key
    ///      // 2. Encrypt transfer amount using recipient's public key
    ///      bytes memory encrypted = encryptAmount(100e18, recipientPubKey);
    ///      // 3. Generate ZK proof
    ///      bytes memory proof = generateTransferProof(myBalance, 100e18, ...);
    ///      // 4. Transfer
    ///      precompile.confidentialTransfer(assetId, recipient, encrypted, proof);
    ///      // 5. Recipient must claim to use the funds
    ///      ```
    ///
    /// @param asset The asset ID (uint128) to transfer
    /// @param to The recipient address (must have set their public key)
    /// @param encryptedAmount The encrypted transfer amount (exactly 64 bytes, encrypted to recipient)
    /// @param proof The ZK proof bytes proving the transfer is valid
    ///
    /// @custom:selector f49a002f
    /// @custom:security Transfer amounts are fully private; only sender and recipient can know the value
    function confidentialTransfer(
        uint128 asset,
        address to,
        bytes calldata encryptedAmount,
        bytes calldata proof
    ) external;

    /// @notice Claims pending confidential transfers received from other users
    /// @dev After receiving a confidential transfer, the recipient must claim it to make the funds
    ///      usable. This two-step process (transfer + claim) allows for batching multiple incoming
    ///      transfers into a single claim operation.
    ///
    ///      **Operation Flow:**
    ///      1. User receives one or more confidential transfers (from `confidentialTransfer()` calls)
    ///      2. Transfers are stored as "pending" and not yet usable
    ///      3. User decrypts the amounts using their private key (off-chain)
    ///      4. User generates a claim proof for the transfers they want to claim
    ///      5. User calls this function to claim, making the funds usable
    ///
    ///      **Why Claims Are Needed:**
    ///      - Allows batching multiple transfers into one claim (gas efficiency)
    ///      - Enables recipient to verify amounts before claiming
    ///      - Provides a clear audit trail of when funds became available
    ///      - Allows selective claiming (can claim specific transfers)
    ///
    ///      **ZK Proof:**
    ///      The proof must contain:
    ///      - Transfer IDs being claimed (identifies which pending transfers)
    ///      - Proof that the claimer is the legitimate recipient
    ///      - Proof that encrypted amounts are correctly decrypted
    ///      - New balance commitment after claiming
    ///
    ///      **Batch Claiming:**
    ///      A single claim can include multiple pending transfers, which is more gas-efficient
    ///      than claiming them one by one.
    ///
    ///      **Gas Cost:** ~80,000-150,000 gas base + ~20,000 gas per additional transfer claimed
    ///
    ///      **Reverts:**
    ///      - If proof verification fails
    ///      - If transfer IDs are invalid or already claimed
    ///      - If caller is not the legitimate recipient
    ///      - If decryption proofs don't match
    ///
    ///      **Example:**
    ///      ```solidity
    ///      // After receiving transfers...
    ///
    ///      // 1. Query pending transfers off-chain (via events or indexer)
    ///      // 2. Decrypt amounts using private key
    ///      // 3. Generate claim proof for transfer IDs
    ///      bytes memory claimProof = generateClaimProof(transferIds, decryptedAmounts);
    ///      // 4. Claim the transfers
    ///      precompile.confidentialClaim(assetId, claimProof);
    ///      // 5. Funds now available in confidential balance
    ///      ```
    ///
    ///      **Integration Notes:**
    ///      - Applications should monitor events to detect incoming transfers
    ///      - Users need their private key to decrypt and claim
    ///      - Consider automatic claiming for better UX
    ///      - Can implement "claim on first action" pattern to batch claims
    ///
    /// @param asset The asset ID (uint128) to claim transfers for
    /// @param proof The ZK proof bytes containing transfer IDs and decryption proofs
    ///
    /// @custom:selector 12cb9d88
    /// @custom:security Claiming requires the private key to decrypt amounts; secure key management is essential
    function confidentialClaim(uint128 asset, bytes calldata proof) external;
}
