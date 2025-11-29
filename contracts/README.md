# Confidential Assets EVM Contracts

This directory contains Solidity contracts for ERC-7984 compatibility with the Confidential Assets precompile.

## Overview

The Confidential Assets precompile provides multi-asset confidential token functionality on Frontier-based EVM chains. These contracts provide an ERC-7984 compliant wrapper that allows standard ERC-7984 consumers to interact with confidential assets.

## Contracts

### Interfaces

- **`interfaces/IERC7984.sol`** - The standard ERC-7984 Confidential Fungible Token interface as defined in [EIP-7984](https://eips.ethereum.org/EIPS/eip-7984)
- **`interfaces/IConfidentialAssets.sol`** - Interface for the Confidential Assets precompile at address `0x800`. This interface includes `@custom:selector` annotations that are verified by Rust tests.

### Main Contracts

- **`ERC7984ConfidentialToken.sol`** - Wrapper contract that adapts the multi-asset precompile to the single-token ERC-7984 interface

### Test Contracts

- **`test/ERC7984Consumer.sol`** - Example consumer contract demonstrating ERC-7984 interface usage

## Interface Verification

The Solidity interface `IConfidentialAssets.sol` is **verified against the Rust precompile** using the `precompile_utils` testing framework. The Rust tests:

1. Parse the Solidity interface file
2. Extract `@custom:selector` annotations
3. Verify the precompile implements each selector

Run the verification tests:

```bash
cargo test -p confidential-assets-evm-precompile
```

Key tests:
- `selectors_match_solidity_interface` - Verifies computed selectors match expected values
- `precompile_matches_solidity_interface_file` - Parses the `.sol` file and verifies all selectors
- `erc7984_wrapper_uses_correct_precompile_selectors` - Verifies the wrapper can call the precompile
- `erc7984_wrapper_integration_scenario` - Simulates complete ERC-7984 consumer flow

This ensures the Solidity interface always matches the actual precompile implementation, and that the ERC-7984 wrapper will work correctly.

## ERC-7984 Compatibility

The wrapper provides the following ERC-7984 functions:

| Function | Status | Notes |
|----------|--------|-------|
| `name()` | ✅ | Returns cached or precompile value |
| `symbol()` | ✅ | Returns cached or precompile value |
| `decimals()` | ✅ | Returns cached or precompile value |
| `confidentialTotalSupply()` | ✅ | Delegates to precompile |
| `confidentialBalanceOf(address)` | ✅ | Delegates to precompile |
| `isOperator(address,address)` | ✅ | Managed by wrapper |
| `setOperator(address,uint48)` | ✅ | Managed by wrapper |
| `confidentialTransfer(address,bytes32)` | ⚠️ | Reverts - use variant with data |
| `confidentialTransfer(address,bytes32,bytes)` | ✅ | data = (encryptedAmount, proof) |
| `confidentialTransferFrom(...)` | ✅ | Requires operator authorization |

### Data Parameter Encoding

For transfer functions, the `bytes data` parameter must be ABI-encoded as:

```solidity
bytes memory data = abi.encode(
    encryptedAmount,  // bytes, exactly 64 bytes
    proof             // bytes, ZK proof
);
```

## Precompile Selectors

The precompile functions and their selectors (verified by Rust tests):

| Function | Selector |
|----------|----------|
| `confidentialBalanceOf(uint128,address)` | `0xcd40095b` |
| `confidentialTotalSupply(uint128)` | `0xefa18641` |
| `name(uint128)` | `0xc624440a` |
| `symbol(uint128)` | `0x117f1264` |
| `decimals(uint128)` | `0x09d2f9b4` |
| `setPublicKey(bytes)` | `0xa91d58b4` |
| `deposit(uint128,uint256,bytes)` | `0x94679bd1` |
| `withdraw(uint128,bytes,bytes)` | `0xf1f9153b` |
| `confidentialTransfer(uint128,address,bytes,bytes)` | `0xf49a002f` |
| `confidentialClaim(uint128,bytes)` | `0x12cb9d88` |

## Usage

### Deploying the Wrapper

```solidity
// Deploy wrapper for asset ID 1
ERC7984ConfidentialToken token = new ERC7984ConfidentialToken(
    1,                      // asset ID
    "Confidential USD",     // name
    "cUSD",                 // symbol
    18                      // decimals
);
```

### Using as ERC-7984 Consumer

```solidity
// Any contract expecting IERC7984 can use the wrapper
IERC7984 token = IERC7984(wrapperAddress);

// Query balance
bytes32 balance = token.confidentialBalanceOf(account);

// Transfer with ZK proof
bytes memory data = abi.encode(encryptedAmount, proof);
token.confidentialTransfer(recipient, commitmentHandle, data);
```

### Setting Up for Transfers

Before receiving confidential transfers, accounts must register their public key:

```solidity
// Set ElGamal public key (64 bytes)
token.setPublicKey(pubkey);
```

After receiving a transfer, recipients must claim pending deposits:

```solidity
// Claim with proof containing transfer IDs
token.claim(claimProof);
```

## Architecture

```
┌─────────────────────────────────────┐
│         ERC-7984 Consumer           │
│   (Any contract using IERC7984)     │
└──────────────┬──────────────────────┘
               │
               │ IERC7984 interface
               ▼
┌─────────────────────────────────────┐
│    ERC7984ConfidentialToken         │
│         (Wrapper Contract)          │
│  - Binds to single asset ID         │
│  - Manages operators                │
│  - Translates interface             │
└──────────────┬──────────────────────┘
               │
               │ IConfidentialAssets
               ▼
┌─────────────────────────────────────┐
│   Confidential Assets Precompile    │
│          (0x800)                    │
│  - Multi-asset support              │
│  - ZK proof verification            │
│  - Encrypted state management       │
└─────────────────────────────────────┘
```

## License

MIT
