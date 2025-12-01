# Custom Ramps

Implement custom on/off ramps between public and confidential assets.

## Overview

The **Ramp** bridges public assets (visible balances) and confidential assets (encrypted balances):

- **Deposit (On-Ramp)**: Public → Confidential
- **Withdraw (Off-Ramp)**: Confidential → Public

## Ramp Trait

```rust
pub trait Ramp<AccountId, AssetId, Balance> {
    type Error: Into<DispatchError>;

    /// Transfer public assets between accounts
    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error>;

    /// Mint public assets (for withdrawals)
    fn mint(
        to: &AccountId,
        asset: &AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error>;

    /// Burn public assets (for deposits)
    fn burn(
        from: &AccountId,
        asset: &AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error>;
}
```

## Standard Implementation

For chains using `pallet-assets` and `pallet-balances`:

```rust
use pallet_assets::Pallet as Assets;
use pallet_balances::Pallet as Balances;
use frame_support::traits::{
    Currency, ExistenceRequirement,
    tokens::fungibles::Mutate,
};

pub const NATIVE_ASSET_ID: AssetId = 0;

pub struct StandardRamp;

impl Ramp<AccountId, AssetId, Balance> for StandardRamp {
    type Error = DispatchError;

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        if asset == NATIVE_ASSET_ID {
            <Balances as Currency<AccountId>>::transfer(
                from,
                to,
                amount,
                ExistenceRequirement::AllowDeath,
            )
        } else {
            <Assets as Mutate<AccountId>>::transfer(
                asset,
                from,
                to,
                amount,
                Preservation::Expendable,
            )
        }
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        if *asset == NATIVE_ASSET_ID {
            let _ = <Balances as Currency<AccountId>>::deposit_creating(to, amount);
            Ok(())
        } else {
            <Assets as Mutate<AccountId>>::mint_into(*asset, to, amount)
        }
    }

    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        if *asset == NATIVE_ASSET_ID {
            let _ = <Balances as Currency<AccountId>>::withdraw(
                from,
                amount,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;
            Ok(())
        } else {
            <Assets as Mutate<AccountId>>::burn_from(
                *asset,
                from,
                amount,
                Preservation::Expendable,
                Precision::BestEffort,
                Fortitude::Polite,
            )?;
            Ok(())
        }
    }
}
```

## Pool-Based Ramp

For systems with liquidity pools:

```rust
pub struct PoolRamp;

impl Ramp<AccountId, AssetId, Balance> for PoolRamp {
    type Error = DispatchError;

    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Transfer to pool instead of burning
        let pool_account = ConfidentialPool::account_id(*asset);

        <Assets as Mutate<AccountId>>::transfer(
            *asset,
            from,
            &pool_account,
            amount,
            Preservation::Expendable,
        )
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Transfer from pool instead of minting
        let pool_account = ConfidentialPool::account_id(*asset);

        <Assets as Mutate<AccountId>>::transfer(
            *asset,
            &pool_account,
            to,
            amount,
            Preservation::Expendable,
        )
    }

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        <Assets as Mutate<AccountId>>::transfer(
            asset,
            from,
            to,
            amount,
            Preservation::Expendable,
        )
    }
}
```

## Fee-Taking Ramp

Add fees to deposits/withdrawals:

```rust
parameter_types! {
    pub const DepositFeePercent: u32 = 1;   // 0.01%
    pub const WithdrawFeePercent: u32 = 5;  // 0.05%
    pub FeeRecipient: AccountId = PalletId(*b"ca_fees!").into_account_truncating();
}

pub struct FeeRamp;

impl Ramp<AccountId, AssetId, Balance> for FeeRamp {
    type Error = DispatchError;

    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Calculate fee
        let fee = amount.saturating_mul(DepositFeePercent::get() as u128) / 10_000;
        let net_amount = amount.saturating_sub(fee);

        // Transfer fee to recipient
        if fee > 0 {
            <Assets as Mutate<AccountId>>::transfer(
                *asset,
                from,
                &FeeRecipient::get(),
                fee,
                Preservation::Expendable,
            )?;
        }

        // Burn net amount
        <Assets as Mutate<AccountId>>::burn_from(
            *asset,
            from,
            net_amount,
            Preservation::Expendable,
            Precision::Exact,
            Fortitude::Polite,
        )?;

        Ok(())
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Calculate fee
        let fee = amount.saturating_mul(WithdrawFeePercent::get() as u128) / 10_000;
        let net_amount = amount.saturating_sub(fee);

        // Mint net amount
        <Assets as Mutate<AccountId>>::mint_into(*asset, to, net_amount)?;

        // Mint fee to recipient
        if fee > 0 {
            <Assets as Mutate<AccountId>>::mint_into(*asset, &FeeRecipient::get(), fee)?;
        }

        Ok(())
    }

    fn transfer_from(
        from: &AccountId,
        to: &AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> Result<(), Self::Error> {
        <Assets as Mutate<AccountId>>::transfer(
            asset,
            from,
            to,
            amount,
            Preservation::Expendable,
        )
    }
}
```

## Rate-Limited Ramp

Implement deposit/withdrawal limits:

```rust
parameter_types! {
    pub const MaxDepositPerBlock: Balance = 1_000_000 * UNIT;
    pub const MaxWithdrawPerBlock: Balance = 500_000 * UNIT;
}

#[pallet::storage]
pub type DepositedThisBlock<T: Config> =
    StorageMap<_, Twox64Concat, T::AssetId, Balance, ValueQuery>;

#[pallet::storage]
pub type WithdrawnThisBlock<T: Config> =
    StorageMap<_, Twox64Concat, T::AssetId, Balance, ValueQuery>;

pub struct RateLimitedRamp;

impl Ramp<AccountId, AssetId, Balance> for RateLimitedRamp {
    type Error = DispatchError;

    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Check rate limit
        let deposited = DepositedThisBlock::<Runtime>::get(asset);
        let new_total = deposited.checked_add(amount)
            .ok_or(ArithmeticError::Overflow)?;

        ensure!(
            new_total <= MaxDepositPerBlock::get(),
            Error::<Runtime>::DepositLimitExceeded
        );

        // Update counter
        DepositedThisBlock::<Runtime>::insert(asset, new_total);

        // Execute burn
        <Assets as Mutate<AccountId>>::burn_from(
            *asset, from, amount,
            Preservation::Expendable,
            Precision::Exact,
            Fortitude::Polite,
        )?;

        Ok(())
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Check rate limit
        let withdrawn = WithdrawnThisBlock::<Runtime>::get(asset);
        let new_total = withdrawn.checked_add(amount)
            .ok_or(ArithmeticError::Overflow)?;

        ensure!(
            new_total <= MaxWithdrawPerBlock::get(),
            Error::<Runtime>::WithdrawLimitExceeded
        );

        // Update counter
        WithdrawnThisBlock::<Runtime>::insert(asset, new_total);

        // Execute mint
        <Assets as Mutate<AccountId>>::mint_into(*asset, to, amount)?;

        Ok(())
    }

    // ... transfer_from implementation
}

// Reset counters each block
#[pallet::hooks]
impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn on_initialize(_: BlockNumberFor<T>) -> Weight {
        DepositedThisBlock::<T>::remove_all(None);
        WithdrawnThisBlock::<T>::remove_all(None);
        Weight::zero()
    }
}
```

## XCM-Based Ramp

For cross-chain asset bridging:

```rust
pub struct XcmRamp;

impl Ramp<AccountId, AssetId, Balance> for XcmRamp {
    type Error = DispatchError;

    fn burn(from: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Lock locally and reserve on reserve chain
        let reserve_location = AssetRegistry::reserve_location(*asset)
            .ok_or(Error::<Runtime>::UnknownAsset)?;

        // Transfer to holding account
        let holding = XcmHolding::account_id();
        <Assets as Mutate<AccountId>>::transfer(
            *asset, from, &holding, amount,
            Preservation::Expendable,
        )?;

        // Emit event for off-chain tracking
        Self::deposit_event(Event::AssetLocked {
            asset: *asset,
            amount,
            from: from.clone(),
        });

        Ok(())
    }

    fn mint(to: &AccountId, asset: &AssetId, amount: Balance) -> Result<(), Self::Error> {
        // Release from holding account
        let holding = XcmHolding::account_id();

        <Assets as Mutate<AccountId>>::transfer(
            *asset, &holding, to, amount,
            Preservation::Expendable,
        )?;

        Self::deposit_event(Event::AssetReleased {
            asset: *asset,
            amount,
            to: to.clone(),
        });

        Ok(())
    }

    // ... transfer_from implementation
}
```

## No-Op Ramp (Testing)

For testing without real asset operations:

```rust
pub struct NoOpRamp;

impl<AccountId, AssetId, Balance> Ramp<AccountId, AssetId, Balance> for NoOpRamp {
    type Error = ();

    fn transfer_from(_: &AccountId, _: &AccountId, _: AssetId, _: Balance) -> Result<(), ()> {
        Ok(())
    }

    fn mint(_: &AccountId, _: &AssetId, _: Balance) -> Result<(), ()> {
        Ok(())
    }

    fn burn(_: &AccountId, _: &AssetId, _: Balance) -> Result<(), ()> {
        Ok(())
    }
}
```

## Configuration

Wire your ramp into the runtime:

```rust
impl pallet_confidential_assets::Config for Runtime {
    type Ramp = StandardRamp;  // Or your custom implementation
    // ...
}
```

## Testing Ramps

```rust
#[test]
fn fee_ramp_takes_correct_fee() {
    new_test_ext().execute_with(|| {
        let initial = Assets::balance(ASSET, &ALICE);
        let deposit_amount = 10_000 * UNIT;
        let expected_fee = deposit_amount / 10_000;  // 0.01%

        // Deposit
        ConfidentialAssets::deposit(
            RuntimeOrigin::signed(ALICE),
            ASSET,
            deposit_amount,
            proof,
        ).unwrap();

        // Check fee recipient received fee
        assert_eq!(
            Assets::balance(ASSET, &FeeRecipient::get()),
            expected_fee
        );

        // Check Alice's balance decreased correctly
        assert_eq!(
            Assets::balance(ASSET, &ALICE),
            initial - deposit_amount
        );
    });
}
```

## Security Considerations

1. **Reentrancy**: Ensure ramp operations are atomic
2. **Overflow**: Use saturating/checked arithmetic
3. **Authorization**: Ramp calls come from the pallet, not users directly
4. **Slippage**: For pool-based ramps, consider price manipulation

## Next Steps

- [Testing Guide](./testing.md) - Test your ramp implementation
- [Client Integration](./client.md) - Handle ramp in your UI
- [API Reference](./api.md) - Complete API documentation
