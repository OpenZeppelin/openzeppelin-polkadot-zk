// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title IConfidentialAssets
/// @dev Interface for the Confidential Assets precompile.
///
/// This precompile provides multi-asset confidential token functionality
/// with ZK-proof based privacy. It supports shielding (deposit) and
/// unshielding (withdraw) of assets between public and confidential balances.
///
/// Deployed at address: 0x0000000000000000000000000000000000000800
///
/// NOTE: The @custom:selector annotations are verified by Rust tests in
/// precompiles/confidential-assets-evm/src/tests.rs to ensure the Solidity
/// interface matches the Rust precompile implementation.
interface IConfidentialAssets {
    /// @dev Returns the encrypted balance commitment for an account.
    /// @param asset The asset ID
    /// @param who The account address
    /// @return The 32-byte balance commitment
    /// @custom:selector cd40095b
    function confidentialBalanceOf(uint128 asset, address who) external view returns (bytes32);

    /// @dev Returns the encrypted total supply commitment for an asset.
    /// @param asset The asset ID
    /// @return The 32-byte total supply commitment
    /// @custom:selector efa18641
    function confidentialTotalSupply(uint128 asset) external view returns (bytes32);

    /// @dev Returns the name of an asset.
    /// @param asset The asset ID
    /// @return The asset name
    /// @custom:selector c624440a
    function name(uint128 asset) external view returns (string memory);

    /// @dev Returns the symbol of an asset.
    /// @param asset The asset ID
    /// @return The asset symbol
    /// @custom:selector 117f1264
    function symbol(uint128 asset) external view returns (string memory);

    /// @dev Returns the decimals of an asset.
    /// @param asset The asset ID
    /// @return The number of decimals
    /// @custom:selector 09d2f9b4
    function decimals(uint128 asset) external view returns (uint8);

    /// @dev Sets the caller's public key for receiving confidential transfers.
    /// @param pubkey The ElGamal public key (64 bytes)
    /// @custom:selector a91d58b4
    function setPublicKey(bytes calldata pubkey) external;

    /// @dev Deposits (shields) public assets into confidential balance.
    /// @param asset The asset ID
    /// @param amount The amount to deposit
    /// @param proof The ZK proof for the deposit
    /// @custom:selector 94679bd1
    function deposit(uint128 asset, uint256 amount, bytes calldata proof) external;

    /// @dev Withdraws (unshields) confidential balance to public assets.
    /// @param asset The asset ID
    /// @param encryptedAmount The encrypted amount (64 bytes)
    /// @param proof The ZK proof for the withdrawal
    /// @custom:selector f1f9153b
    function withdraw(uint128 asset, bytes calldata encryptedAmount, bytes calldata proof) external;

    /// @dev Performs a confidential transfer.
    /// @param asset The asset ID
    /// @param to The recipient address
    /// @param encryptedAmount The encrypted transfer amount (64 bytes)
    /// @param proof The ZK proof for the transfer
    /// @custom:selector f49a002f
    function confidentialTransfer(
        uint128 asset,
        address to,
        bytes calldata encryptedAmount,
        bytes calldata proof
    ) external;

    /// @dev Claims pending confidential deposits.
    /// @param asset The asset ID
    /// @param proof The ZK proof containing transfer IDs to claim
    /// @custom:selector 12cb9d88
    function confidentialClaim(uint128 asset, bytes calldata proof) external;
}
