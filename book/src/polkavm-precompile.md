# PolkaVM Precompile

The Confidential Assets PolkaVM Precompile exposes confidential assets functionality to smart contracts running on pallet-revive. This enables Solidity developers to interact with confidential balances directly from their contracts.

## Precompile Address

The precompile is registered at address:

```
0x0000000000000000000000000000000C010000
```

The `0x0C01` prefix follows the convention "C01" = "Confidential 01".

## Solidity Interface

```solidity
interface IConfidentialAssets {
    /// @notice Get the encrypted balance commitment for an account
    /// @param assetId The asset identifier
    /// @param account The account address (as bytes32)
    /// @return The balance commitment as bytes32
    function confidentialBalance(uint128 assetId, bytes32 account) external view returns (bytes32);

    /// @notice Get the public key registered for an account
    /// @param account The account address (as bytes32)
    /// @return The public key as bytes32 (zero if not registered)
    function publicKey(bytes32 account) external view returns (bytes32);

    /// @notice Get the total supply commitment for an asset
    /// @param assetId The asset identifier
    /// @return The total supply commitment as bytes32
    function totalSupply(uint128 assetId) external view returns (bytes32);
}
```

## Usage Example

```solidity
pragma solidity ^0.8.0;

interface IConfidentialAssets {
    function confidentialBalance(uint128 assetId, bytes32 account) external view returns (bytes32);
    function publicKey(bytes32 account) external view returns (bytes32);
    function totalSupply(uint128 assetId) external view returns (bytes32);
}

contract ConfidentialAssetsClient {
    IConfidentialAssets constant CONFIDENTIAL_ASSETS =
        IConfidentialAssets(0x0000000000000000000000000000000C010000);

    function getBalance(uint128 assetId, bytes32 account) external view returns (bytes32) {
        return CONFIDENTIAL_ASSETS.confidentialBalance(assetId, account);
    }

    function hasPublicKey(bytes32 account) external view returns (bool) {
        bytes32 pk = CONFIDENTIAL_ASSETS.publicKey(account);
        return pk != bytes32(0);
    }
}
```

## Function Selectors

| Function | Selector |
|----------|----------|
| `confidentialBalance(uint128,address)` | `0x4c5b3e9d` |
| `publicKey(address)` | `0x685e3b40` |
| `totalSupply(uint128)` | `0x18160ddd` |

## Runtime Configuration

To use this precompile, your runtime must:

1. Include `pallet-revive` with the precompile registered
2. Include `pallet-confidential-assets` for balance storage
3. Include `pallet-zkhe` for public key management

The precompile is configured in the runtime via the `Precompiles` type:

```rust
use confidential_assets_revive_precompile::ConfidentialAssetsPrecompile;

type Precompiles = (
    ConfidentialAssetsPrecompile<Runtime>,
    // ... other precompiles
);
```

## Security Considerations

- **View-only functions**: Currently, only read operations are exposed. State-changing operations (transfers, minting) must be performed through extrinsics.
- **Commitment privacy**: The returned values are cryptographic commitments, not plaintext balances. Only the account holder with the corresponding private key can decrypt them.
- **Zero public key**: A zero return from `publicKey()` indicates the account has not registered a public key. Contracts should handle this case appropriately.
