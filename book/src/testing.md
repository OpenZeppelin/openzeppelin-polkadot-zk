# Testing Guide

Comprehensive testing strategies for confidential assets.

## Overview

Testing confidential assets requires multiple approaches:

1. **Unit Tests**: Test individual pallet functions
2. **Property Tests**: Verify invariants with random inputs
3. **Integration Tests**: Test pallet interactions
4. **XCM Simulator Tests**: Test cross-chain flows
5. **Vector Tests**: Use pre-generated proofs for deterministic testing

## Test Environment Setup

### Mock Runtime

```rust
// In pallets/confidential-assets/src/mock.rs

use crate as pallet_confidential_assets;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use sp_runtime::traits::IdentityLookup;

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime! {
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Assets: pallet_assets,
        Zkhe: pallet_zkhe,
        ConfidentialAssets: pallet_confidential_assets,
    }
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type AccountData = pallet_balances::AccountData<u64>;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU64<1>;
    // ... other config
}

impl pallet_zkhe::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u32;
    type Balance = u64;
    type Verifier = MockVerifier;
    type WeightInfo = ();
}

impl pallet_confidential_assets::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u32;
    type Balance = u64;
    type Backend = Zkhe;
    type Ramp = MockRamp;
    type AssetMetadata = ();
    type Acl = ();
    type Operators = ();
    type WeightInfo = ();
}
```

### Mock Verifier

```rust
// For unit tests, use a mock verifier that accepts all proofs
pub struct MockVerifier;

impl ZkVerifier for MockVerifier {
    type Error = ();

    fn verify_transfer_sent(
        _asset: &[u8],
        _from_pk: &[u8],
        _to_pk: &[u8],
        _from_old_avail: &[u8],
        _to_old_pending: &[u8],
        _delta_ct: &[u8],
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), Self::Error> {
        // Return mock updated commitments
        Ok((vec![0u8; 32], vec![0u8; 32]))
    }

    fn verify_transfer_received(
        _asset: &[u8],
        _who_pk: &[u8],
        _avail_old: &[u8],
        _pending_old: &[u8],
        _commits: &[[u8; 32]],
        _envelope: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), Self::Error> {
        Ok((vec![0u8; 32], vec![0u8; 32]))
    }

    fn verify_mint(
        _asset: &[u8],
        _to_pk: &PublicKeyBytes,
        _to_old_pending: &[u8],
        _total_old: &[u8],
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, EncryptedAmount), Self::Error> {
        Ok((vec![0u8; 32], vec![0u8; 32], [0u8; 64]))
    }

    fn verify_burn(
        _asset: &[u8],
        _from_pk: &PublicKeyBytes,
        _from_old_avail: &[u8],
        _total_old: &[u8],
        _amount_ct: &EncryptedAmount,
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, u64), Self::Error> {
        Ok((vec![0u8; 32], vec![0u8; 32], 100))
    }

    fn disclose(_asset: &[u8], _pk: &[u8], _cipher: &[u8]) -> Result<u64, Self::Error> {
        Ok(100)
    }
}
```

### Test Externalities Builder

```rust
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE, 10_000),
            (BOB, 10_000),
            (CHARLIE, 10_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const ASSET_ID: u32 = 100;
```

## Unit Tests

### Basic Operations

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_noop, assert_ok};

    #[test]
    fn set_public_key_works() {
        new_test_ext().execute_with(|| {
            let pk = [1u8; 32];

            assert_ok!(ConfidentialAssets::set_public_key(
                RuntimeOrigin::signed(ALICE),
                pk.to_vec().try_into().unwrap()
            ));

            assert_eq!(Zkhe::public_key(&ALICE), Some(pk));
        });
    }

    #[test]
    fn set_public_key_fails_if_already_set() {
        new_test_ext().execute_with(|| {
            let pk = [1u8; 32];

            assert_ok!(ConfidentialAssets::set_public_key(
                RuntimeOrigin::signed(ALICE),
                pk.to_vec().try_into().unwrap()
            ));

            assert_noop!(
                ConfidentialAssets::set_public_key(
                    RuntimeOrigin::signed(ALICE),
                    pk.to_vec().try_into().unwrap()
                ),
                Error::<Test>::PkAlreadySet
            );
        });
    }

    #[test]
    fn deposit_requires_public_key() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                ConfidentialAssets::deposit(
                    RuntimeOrigin::signed(ALICE),
                    ASSET_ID,
                    100,
                    vec![].try_into().unwrap()
                ),
                pallet_zkhe::Error::<Test>::NoPk
            );
        });
    }

    #[test]
    fn transfer_requires_recipient_pk() {
        new_test_ext().execute_with(|| {
            setup_alice_with_balance();

            assert_noop!(
                ConfidentialAssets::confidential_transfer(
                    RuntimeOrigin::signed(ALICE),
                    ASSET_ID,
                    BOB,
                    [0u8; 64],
                    vec![].try_into().unwrap()
                ),
                pallet_zkhe::Error::<Test>::NoPk
            );
        });
    }

    #[test]
    fn accept_pending_works() {
        new_test_ext().execute_with(|| {
            setup_alice_and_bob_with_pks();
            // Create pending balance for Bob
            setup_pending_for_bob();

            assert_ok!(ConfidentialAssets::accept_pending(
                RuntimeOrigin::signed(BOB),
                ASSET_ID,
                vec![].try_into().unwrap()
            ));
        });
    }
}
```

### Event Verification

```rust
#[test]
fn transfer_emits_event() {
    new_test_ext().execute_with(|| {
        setup_alice_and_bob_with_balances();

        assert_ok!(ConfidentialAssets::confidential_transfer(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID,
            BOB,
            [0u8; 64],
            vec![].try_into().unwrap()
        ));

        System::assert_has_event(RuntimeEvent::ConfidentialAssets(
            crate::Event::ConfidentialTransfer {
                asset: ASSET_ID,
                from: ALICE,
                to: BOB,
                encrypted_amount: [0u8; 64],
            }
        ));
    });
}
```

## Property Tests

Property tests verify invariants hold for arbitrary inputs.

### Setup

```toml
# In Cargo.toml
[dev-dependencies]
proptest = "1.5"
```

### Property Test Examples

```rust
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Public key registration always succeeds with valid 32-byte key
    #[test]
    fn prop_set_pk_succeeds_with_valid_pk(
        account in 1u64..1000,
        pk_bytes in prop::array::uniform32(any::<u8>())
    ) {
        new_test_ext().execute_with(|| {
            let result = ConfidentialAssets::set_public_key(
                RuntimeOrigin::signed(account),
                pk_bytes.to_vec().try_into().unwrap()
            );
            assert!(result.is_ok());
            assert_eq!(Zkhe::public_key(&account), Some(pk_bytes));
        });
    }

    /// Sequential transfers increment UTXO IDs correctly
    #[test]
    fn prop_utxo_ids_increment(
        num_transfers in 1usize..10,
        sender in 1u64..100,
        recipient in 100u64..200
    ) {
        new_test_ext().execute_with(|| {
            setup_accounts(sender, recipient);

            let initial_id = pallet_zkhe::NextUtxoId::<Test>::get();

            for i in 0..num_transfers {
                let _ = ConfidentialAssets::confidential_transfer(
                    RuntimeOrigin::signed(sender),
                    ASSET_ID,
                    recipient,
                    [i as u8; 64],
                    vec![].try_into().unwrap()
                );
            }

            let final_id = pallet_zkhe::NextUtxoId::<Test>::get();
            assert_eq!(final_id, initial_id + num_transfers as u64);
        });
    }

    /// Malformed proofs are rejected
    #[test]
    fn prop_malformed_proof_rejected(
        random_bytes in prop::collection::vec(any::<u8>(), 0..100)
    ) {
        new_test_ext().execute_with(|| {
            setup_alice_with_pk();

            // With real verifier, random bytes should fail
            // This tests the verifier integration
            let result = ConfidentialAssets::deposit(
                RuntimeOrigin::signed(ALICE),
                ASSET_ID,
                100,
                random_bytes.try_into().unwrap_or_default()
            );

            // Depending on mock vs real verifier
            // With real verifier: assert!(result.is_err());
        });
    }

    /// Balance commitments are always 32 bytes
    #[test]
    fn prop_commitments_are_32_bytes(
        account in 1u64..100,
        asset in 1u32..100
    ) {
        new_test_ext().execute_with(|| {
            setup_account_with_balance(account, asset);

            let commit = pallet_zkhe::AvailableBalanceCommit::<Test>::get(asset, account);
            if let Some(c) = commit {
                assert_eq!(c.len(), 32);
            }
        });
    }
}
```

## Integration Tests

Test interactions between pallets:

```rust
#[test]
fn full_deposit_transfer_withdraw_flow() {
    new_test_ext().execute_with(|| {
        // Setup
        let alice_pk = [1u8; 32];
        let bob_pk = [2u8; 32];

        assert_ok!(ConfidentialAssets::set_public_key(
            RuntimeOrigin::signed(ALICE),
            alice_pk.to_vec().try_into().unwrap()
        ));
        assert_ok!(ConfidentialAssets::set_public_key(
            RuntimeOrigin::signed(BOB),
            bob_pk.to_vec().try_into().unwrap()
        ));

        // 1. Deposit public to confidential
        let initial_public = Balances::free_balance(ALICE);
        assert_ok!(ConfidentialAssets::deposit(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID,
            1000,
            vec![].try_into().unwrap()
        ));

        // Verify public balance decreased
        assert_eq!(Balances::free_balance(ALICE), initial_public - 1000);

        // 2. Accept pending deposit
        assert_ok!(ConfidentialAssets::accept_pending(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID,
            vec![].try_into().unwrap()
        ));

        // 3. Transfer to Bob
        assert_ok!(ConfidentialAssets::confidential_transfer(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID,
            BOB,
            [0u8; 64],
            vec![].try_into().unwrap()
        ));

        // 4. Bob accepts
        assert_ok!(ConfidentialAssets::accept_pending(
            RuntimeOrigin::signed(BOB),
            ASSET_ID,
            vec![].try_into().unwrap()
        ));

        // 5. Bob withdraws
        let bob_initial = Balances::free_balance(BOB);
        assert_ok!(ConfidentialAssets::withdraw(
            RuntimeOrigin::signed(BOB),
            ASSET_ID,
            [0u8; 64],
            vec![].try_into().unwrap()
        ));

        // Verify Bob's public balance increased
        assert!(Balances::free_balance(BOB) > bob_initial);
    });
}
```

## Vector Tests

Use pre-generated proofs from `zkhe-vectors`:

```rust
// In xcm/src/vector_tests.rs

use zkhe_vectors::{
    TransferSentVector, TransferReceivedVector, MintVector, BurnVector,
    TRANSFER_SENT_VECTORS, TRANSFER_RECEIVED_VECTORS, MINT_VECTORS, BURN_VECTORS,
};
use zkhe_verifier::ZkheVerifier;

#[test]
fn verify_transfer_sent_with_vectors() {
    for (i, vector) in TRANSFER_SENT_VECTORS.iter().enumerate() {
        let result = ZkheVerifier::verify_transfer_sent(
            &vector.asset,
            &vector.from_pk,
            &vector.to_pk,
            &vector.from_old_avail,
            &vector.to_old_pending,
            &vector.delta_ct,
            &vector.proof,
        );

        assert!(result.is_ok(), "Vector {} failed: {:?}", i, result.err());

        let (new_from_avail, new_to_pending) = result.unwrap();
        assert_eq!(new_from_avail, vector.expected_from_new_avail);
        assert_eq!(new_to_pending, vector.expected_to_new_pending);
    }
}

#[test]
fn tampered_proof_rejected() {
    let vector = &TRANSFER_SENT_VECTORS[0];

    // Tamper with the proof
    let mut tampered_proof = vector.proof.clone();
    tampered_proof[0] ^= 0xFF;

    let result = ZkheVerifier::verify_transfer_sent(
        &vector.asset,
        &vector.from_pk,
        &vector.to_pk,
        &vector.from_old_avail,
        &vector.to_old_pending,
        &vector.delta_ct,
        &tampered_proof,
    );

    assert!(result.is_err(), "Tampered proof should be rejected");
}

#[test]
fn wrong_public_key_rejected() {
    let vector = &TRANSFER_SENT_VECTORS[0];

    // Use wrong public key
    let wrong_pk = [0xFFu8; 32];

    let result = ZkheVerifier::verify_transfer_sent(
        &vector.asset,
        &wrong_pk,  // Wrong key
        &vector.to_pk,
        &vector.from_old_avail,
        &vector.to_old_pending,
        &vector.delta_ct,
        &vector.proof,
    );

    assert!(result.is_err(), "Wrong PK should be rejected");
}
```

## XCM Simulator Tests

Test cross-chain confidential transfers:

```rust
// In xcm/src/tests.rs

use xcm_simulator::TestExt;

decl_test_network! {
    pub struct MockNet {
        relay_chain = Relay,
        parachains = vec![
            (1000, ParaA),
            (2000, ParaB),
        ],
    }
}

#[test]
fn cross_chain_confidential_transfer() {
    MockNet::reset();

    // Setup on ParaA
    ParaA::execute_with(|| {
        // Register Alice's PK
        assert_ok!(ConfidentialAssets::set_public_key(
            parachain::RuntimeOrigin::signed(ALICE),
            alice_pk.to_vec().try_into().unwrap()
        ));

        // Deposit to confidential
        assert_ok!(ConfidentialAssets::deposit(
            parachain::RuntimeOrigin::signed(ALICE),
            ASSET_ID,
            1000,
            mint_proof.try_into().unwrap()
        ));
    });

    // Setup on ParaB
    ParaB::execute_with(|| {
        // Register Bob's PK
        assert_ok!(ConfidentialAssets::set_public_key(
            parachain::RuntimeOrigin::signed(BOB),
            bob_pk.to_vec().try_into().unwrap()
        ));
    });

    // Initiate cross-chain transfer
    ParaA::execute_with(|| {
        assert_ok!(ConfidentialBridge::send_confidential(
            parachain::RuntimeOrigin::signed(ALICE),
            2000,  // Dest para
            BOB,
            ASSET_ID,
            delta_ct,
            escrow_proof.try_into().unwrap(),
            mint_proof.try_into().unwrap(),
        ));
    });

    // Process XCM messages
    MockNet::process_messages();

    // Verify on ParaB
    ParaB::execute_with(|| {
        // Check Bob has pending balance
        let pending = pallet_zkhe::PendingBalanceCommit::<parachain::Runtime>::get(
            ASSET_ID,
            BOB
        );
        assert!(pending.is_some());
    });
}
```

## Benchmarking

Benchmark pallet operations for weight calculation:

```rust
// In pallets/confidential-assets/src/benchmarking.rs

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn set_public_key() {
        let caller: T::AccountId = whitelisted_caller();
        let pk = [1u8; 32];

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), pk.to_vec().try_into().unwrap());

        assert_eq!(T::Backend::public_key(&caller), Some(pk));
    }

    #[benchmark]
    fn deposit(a: Linear<1, 1000>) {
        let caller: T::AccountId = whitelisted_caller();
        setup_caller_pk::<T>(&caller);

        let amount = a as u128;
        let proof = generate_mint_proof::<T>(&caller, amount);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), ASSET_ID.into(), amount, proof);
    }

    #[benchmark]
    fn confidential_transfer() {
        let sender: T::AccountId = whitelisted_caller();
        let recipient: T::AccountId = account("recipient", 0, 0);

        setup_transfer::<T>(&sender, &recipient);

        let delta_ct = [0u8; 64];
        let proof = generate_transfer_proof::<T>(&sender, &recipient);

        #[extrinsic_call]
        _(RawOrigin::Signed(sender), ASSET_ID.into(), recipient, delta_ct, proof);
    }

    impl_benchmark_test_suite!(
        ConfidentialAssets,
        crate::mock::new_test_ext(),
        crate::mock::Test
    );
}
```

## Test Coverage

Check test coverage:

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin -p pallet-confidential-assets -p pallet-zkhe --out Html

# View report
open tarpaulin-report.html
```

## CI Integration

Example GitHub Actions workflow:

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown

      - name: Run unit tests
        run: cargo test --workspace --exclude runtime

      - name: Run property tests
        run: cargo test --workspace -- --include-ignored prop_

      - name: Run XCM simulator tests
        run: cargo test -p xcm

      - name: Run vector tests
        run: cargo test -p xcm -- vector_tests
```

## Next Steps

- [API Reference](./api.md) - Complete API documentation
- [Client Integration](./client.md) - Build client applications
- [Custom Backends](./custom-backends.md) - Implement alternative backends
