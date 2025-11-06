//! XCM Integration Tests for Cross Chain Confidential Transfers
//!
//! Tests include:
//! - XCM reserve transfer of plaintext asset from AssetHub (Asset Hub) -> ConfidentialHub (Confidential Hub)

use log::info;
use polkadot_sdk::{staging_xcm as xcm, *};
use xcm_emulator::*;

use asset_hub_runtime as para_a;
use confidential_runtime as para_b;
use emulated_integration_tests_common::{
    impl_accounts_helpers_for_parachain, impl_accounts_helpers_for_relay_chain,
    impl_assert_events_helpers_for_parachain, impl_assert_events_helpers_for_relay_chain,
    impl_hrmp_channels_helpers_for_relay_chain, impl_send_transact_helpers_for_relay_chain,
    impl_xcm_helpers_for_parachain, impls::Parachain,
};
use frame_support::assert_ok;
use relay_runtime as relay;
use relay_runtime::BuildStorage;

// Handy aliases for clarity
type Balance = parachains_common::Balance;
type AccountId = parachains_common::AccountId;

// ---------------------- Genesis helpers ----------------------
// We keep genesis minimal and do funding with a root call right before sending XCM.

fn relay_genesis() -> sp_core::storage::Storage {
    relay::RuntimeGenesisConfig::default()
        .build_storage()
        .expect("relay genesis storage")
}

fn para_a_genesis() -> sp_core::storage::Storage {
    para_a::RuntimeGenesisConfig::default()
        .build_storage()
        .expect("para A genesis storage")
}

fn para_b_genesis() -> sp_core::storage::Storage {
    para_b::RuntimeGenesisConfig::default()
        .build_storage()
        .expect("para B genesis storage")
}

// ---------------------- Relay definition ----------------------

decl_test_relay_chains! {
    #[api_version(13)]
    pub struct LocalRelay {
        genesis = relay_genesis(),
        on_init = {},
        runtime = relay,
        core = {
            SovereignAccountOf: relay::LocationConverter,
        },
        pallets = {
            Sudo: relay::Sudo,
            Balances: relay::Balances,
            XcmPallet: relay::XcmPallet,
            MessageQueue: relay::MessageQueue,
            Hrmp: relay::Hrmp,
        }
    }
}

impl_accounts_helpers_for_relay_chain!(LocalRelay);
impl_assert_events_helpers_for_relay_chain!(LocalRelay);
impl_hrmp_channels_helpers_for_relay_chain!(LocalRelay);
impl_send_transact_helpers_for_relay_chain!(LocalRelay);

// ---------------------- Parachain definitions ----------------------

decl_test_parachains! {
    pub struct AssetHub {
        genesis = para_a_genesis(),
        on_init = {
            <para_a::AuraExt as frame_support::traits::OnInitialize<u32>>::on_initialize(1);
        },
        runtime = para_a,
        core = {
            // Accept and route XCMP/HrMP on the parachain:
            XcmpMessageHandler: para_a::XcmpQueue,
            // Convert MultiLocation -> AccountId
            LocationToAccountId: para_a::configs::LocationToAccountId,
            // Must return this para's id
            ParachainInfo: para_a::ParachainInfo,
            // Message origin type for MQ on parachain side
            MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
            // Optional digest provider
            DigestProvider: (),
        },
        pallets = {
            PolkadotXcm: para_a::PolkadotXcm,
            System: para_a::System,
            Balances: para_a::Balances,
            Assets: para_a::Assets,
            ForeignAssets: para_a::ForeignAssets,
        }
    },
    pub struct ConfidentialHub {
        genesis = para_b_genesis(),
        on_init = {
            <para_a::AuraExt as frame_support::traits::OnInitialize<u32>>::on_initialize(1);
        },
        runtime = para_b,
        core = {
            // Accept and route XCMP/HrMP on the parachain:
            XcmpMessageHandler: para_b::XcmpQueue,
            // Convert MultiLocation -> AccountId
            LocationToAccountId: para_b::configs::LocationToAccountId,
            // Must return this para's id
            ParachainInfo: para_b::ParachainInfo,
            // Message origin type for MQ on parachain side
            MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
            // Optional digest provider
            DigestProvider: (),
        },
        pallets = {
            PolkadotXcm: para_b::PolkadotXcm,
            System: para_b::System,
            Balances: para_b::Balances,
            Assets: para_b::Assets,
        }
    }
}

// AssetHub helpers
impl_accounts_helpers_for_parachain!(AssetHub);
impl_assert_events_helpers_for_parachain!(AssetHub);
impl_xcm_helpers_for_parachain!(AssetHub);

// ConfidentialHub helpers
impl_accounts_helpers_for_parachain!(ConfidentialHub);
impl_assert_events_helpers_for_parachain!(ConfidentialHub);
impl_xcm_helpers_for_parachain!(ConfidentialHub);

// ---------------------- Network (relay + two paras) ----------------------

decl_test_networks! {
    pub struct LocalNet {
        relay_chain = LocalRelay,
        parachains = vec![
            AssetHub,
            ConfidentialHub,
        ],
        bridge = ()
    }
}

//type Relay = LocalRelay<LocalNet>;
type TransparentP = AssetHub<LocalNet>;
type EncryptedP = ConfidentialHub<LocalNet>;

// ---------------------- Small helpers ----------------------

// same ids on both para so one id function
fn id(name: &str) -> AccountId {
    <TransparentP as Chain>::account_id_of(name)
}

fn free_balance_a(who: &AccountId) -> Balance {
    <TransparentP as Chain>::account_data_of(who.clone()).free
}

fn free_balance_b(who: &AccountId) -> Balance {
    <EncryptedP as Chain>::account_data_of(who.clone()).free
}

// ---------------------- Demo flow ----------------------

fn main() {
    // (Optional) set RUST_LOG=info to see emulator event logs.
    // env_logger::init(); // If you add env_logger to Cargo.toml

    // 1) Pick participants & amount
    let alice = id("Alice");
    let bob = id("Bob"); // same id on both paras
    let amount: Balance = 1_000_000_000_000;

    // 2) Fund Alice on Para A (root call so we avoid genesis fuss)
    TransparentP::execute_with(|| {
        // Force set Alice's free balance on Para A
        pallet_balances::Pallet::<para_a::Runtime>::force_set_balance(
            para_a::RuntimeOrigin::root(),
            alice.clone().into(),
            amount * 10, // give her more than she'll send
        )
        .expect("force_set_balance ok");
        // TODO: Force set Alice's free balance on Para A for Reserve Asset of Para B
    });

    // Show initial balances
    println!("== Before ==");
    let before_alice_balance = free_balance_a(&alice);
    println!("AssetHub - Alice: {}", before_alice_balance);
    let before_bob_balance = free_balance_b(&bob);
    println!("ConfidentialHub - Bob  : {}", before_bob_balance);

    // 3) Build a reserve transfer from Para A -> Para B of the native asset from A
    //    (Asset Hub runtimes accept reserve transfer; fee asset item = 0; unlimited weight)
    use xcm::latest::prelude::*;

    let dest = EncryptedP::sibling_location_of(EncryptedP::para_id()); // Parent, Parachain(B)
    let beneficiary: Location = Location::new(
        0, // parents
        [Junction::AccountId32 {
            network: None,
            id: sp_core::crypto::AccountId32::from(bob.clone()).into(),
        }],
    );
    let fee = 10_000_000; // enough to cover BuyExecution on B

    let assets: Assets = vec![
        (dest.clone(), 1).into(), // index 0 — fee asset recognized on B
        (Here, amount).into(),    // index 1 — the reserved asset (Para A native)
    ]
    .into();

    // 4) Dispatch the XCM from Para A (signed by Alice)
    TransparentP::execute_with(|| {
        let v_dest = xcm::VersionedLocation::from(dest.clone());
        let v_beneficiary = xcm::VersionedLocation::from(beneficiary.clone());
        let v_assets = xcm::VersionedAssets::from(assets.clone());
        let origin = para_a::RuntimeOrigin::signed(alice.clone());
        assert_ok!(para_a::PolkadotXcm::transfer_assets(
            origin,
            bx!(v_dest),
            bx!(v_beneficiary),
            bx!(v_assets),
            0, // pay fees with assets[0] (relay)
            None.into(),
        ));
    });

    // Process HRMP/UMP/XCMP once we're OUT of the AssetHub externalities.
    // Any chain's `execute_with` will pump the queues; the relay is standard.
    EncryptedP::assert_xcmp_queue_success(None);

    // 5) One `execute_with` on a parachain triggers HRMP/UMP routing in the emulator.
    //    By here, messages are processed; assert/print final balances.
    //
    // // Show initial balances
    println!("\n== After ==");
    let after_alice_balance = free_balance_a(&alice);
    println!("AssetHub - Alice: {}", before_alice_balance);
    let after_bob_balance = free_balance_b(&bob);
    println!("ConfidentialHub - Bob  : {}", before_bob_balance);

    // After (emulator processed message queues already)
    println!(
        "\n== AFTER ==\nParaA::Alice={}\nParaB::Bob  ={}",
        before_alice_balance, before_bob_balance
    );
    println!(
        "\n== DELTA ==\nA sent: {}\nB recv: {}",
        before_alice_balance.saturating_sub(after_alice_balance),
        after_bob_balance.saturating_sub(before_bob_balance)
    );

    if after_bob_balance <= before_bob_balance {
        panic!("Bob did not receive funds");
    }
    info!("Reserve transfer complete.");
}
