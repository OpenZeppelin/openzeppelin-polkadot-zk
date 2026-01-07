# Custom Ramps

Implement custom on/off ramps between public and confidential assets.

## Overview

The **Ramp** bridges public assets (visible balances) and confidential assets (encrypted balances):

- **Deposit (On-Ramp)**: Public to Confidential
- **Withdraw (Off-Ramp)**: Confidential to Public

## Ramp Trait

Your ramp must implement three methods:

### transfer_from
Transfer public assets between accounts. Used for internal operations.

### mint
Mint/credit public assets to an account. Called during withdrawals to convert confidential balance back to public.

### burn
Burn/debit public assets from an account. Called during deposits to convert public balance to confidential.

## Common Ramp Patterns

### Standard Ramp
The default implementation for chains using `pallet-assets` and `pallet-balances`:
- Uses `Currency` trait for native token operations
- Uses `fungibles::Mutate` trait for other assets
- Directly burns on deposit, mints on withdraw

### Pool-Based Ramp
For systems with liquidity pools:
- Transfers to/from a pool account instead of minting/burning
- Useful when assets have fixed supply
- Pool must be pre-funded with sufficient liquidity

### Fee-Taking Ramp
Adds fees to deposits/withdrawals:
- Calculate fee as percentage of amount
- Transfer fee to designated recipient
- Process net amount for the operation

### Rate-Limited Ramp
Implements deposit/withdrawal limits:
- Track per-block deposit/withdrawal totals per asset
- Reject operations exceeding limits
- Reset counters each block via `on_initialize` hook

### XCM-Based Ramp
For cross-chain asset bridging:
- Lock assets locally on deposit
- Release from holding account on withdraw
- Coordinates with remote chain via XCM messages

### No-Op Ramp
For testing without real asset operations:
- All methods return `Ok(())`
- Useful for testing pallet logic in isolation

## Configuration

Configure your ramp in the runtime:

```rust
impl pallet_confidential_assets::Config for Runtime {
    type Ramp = StandardRamp;  // Or your custom implementation
    // ...
}
```

## Security Considerations

1. **Reentrancy**: Ensure ramp operations are atomic
2. **Overflow**: Use saturating/checked arithmetic
3. **Authorization**: Ramp calls come from the pallet, not users directly
4. **Slippage**: For pool-based ramps, consider price manipulation risks

## Next Steps

- [Testing Guide](./testing.md) - Test your ramp implementation
- [Client Integration](./client.md) - Handle ramp in your UI
- [API Reference](./api.md) - Complete API documentation
