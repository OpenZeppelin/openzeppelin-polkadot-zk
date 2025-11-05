//! XCM Integration Tests for Cross Chain Confidential Transfers
//!
//! Tests include:
//! - XCM reserve transfer of plaintext asset from AssetHub (Asset Hub) -> ConfidentialHub (Confidential Hub)

use log::info;
use polkadot_sdk::{staging_parachain_info as parachain_info, staging_xcm as xcm, *};
use xcm_emulator::*;

use asset_hub_runtime as para_a;
use confidential_runtime as para_b;
use frame_support::assert_ok;
use relay_runtime as relay;
use relay_runtime::BuildStorage;
use xcm::latest::*;
use xcm::{VersionedAssets, VersionedLocation};

macro_rules! bx {
    ($x:expr) => {
        Box::new($x)
    };
}

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
            XcmPallet: relay::Xcm,
            MessageQueue: relay::MessageQueue,
            Hrmp: relay::Hrmp,
        }
    }
}

// ---------------------- Parachain definitions ----------------------

decl_test_parachains! {
    pub struct AssetHub {
        genesis = para_a_genesis(),
        on_init = {},
        runtime = para_a,
        core = {
            // Accept and route XCMP/HrMP on the parachain:
            XcmpMessageHandler: cumulus_pallet_xcmp_queue::Pallet<para_a::Runtime>,
            // Convert MultiLocation -> AccountId
            LocationToAccountId: para_a::configs::LocationToAccountId,
            // Must return this para's id
            ParachainInfo: parachain_info::Pallet<para_a::Runtime>,
            // Message origin type for MQ on parachain side
            MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
            // Optional digest provider; omit to use default `()`
        },
        pallets = {
            System: frame_system::Pallet<para_a::Runtime>,
            Balances: pallet_balances::Pallet<para_a::Runtime>,
            Assets: pallet_assets::Pallet<para_a::Runtime>,           // Asset Hub usually includes this
            MessageQueue: pallet_message_queue::Pallet<para_a::Runtime>,
            XcmpQueue: cumulus_pallet_xcmp_queue::Pallet<para_a::Runtime>,
            XcmPallet: pallet_xcm::Pallet<para_a::Runtime>,
        }
    },
    pub struct ConfidentialHub {
        genesis = para_b_genesis(),
        on_init = {},
        runtime = para_b,
        core = {
            XcmpMessageHandler: cumulus_pallet_xcmp_queue::Pallet<para_b::Runtime>,
            LocationToAccountId: para_b::configs::LocationToAccountId,
            ParachainInfo: parachain_info::Pallet<para_b::Runtime>,
            MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
        },
        pallets = {
            System: frame_system::Pallet<para_b::Runtime>,
            Balances: pallet_balances::Pallet<para_b::Runtime>,
            Assets: pallet_assets::Pallet<para_b::Runtime>,
            MessageQueue: pallet_message_queue::Pallet<para_b::Runtime>,
            XcmpQueue: cumulus_pallet_xcmp_queue::Pallet<para_b::Runtime>,
            XcmPallet: pallet_xcm::Pallet<para_b::Runtime>,
        }
    }
}

use emulated_integration_tests_common::{
    impl_accounts_helpers_for_parachain, impl_assert_events_helpers_for_parachain,
    impl_assets_helpers_for_parachain, impl_assets_helpers_for_system_parachain,
    impl_bridge_helpers_for_chain, impl_foreign_assets_helpers_for_parachain,
    impl_xcm_helpers_for_parachain, impls::Parachain,
};

// AssetHub helpers
impl_accounts_helpers_for_parachain!(AssetHub);
impl_assert_events_helpers_for_parachain!(AssetHub);
impl_assets_helpers_for_parachain!(AssetHub);
impl_xcm_helpers_for_parachain!(AssetHub);

// ConfidentialHub helpers
impl_accounts_helpers_for_parachain!(ConfidentialHub);
impl_assert_events_helpers_for_parachain!(ConfidentialHub);
impl_assets_helpers_for_parachain!(ConfidentialHub);
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

type Relay = LocalRelay<LocalNet>;
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
        use frame_support::traits::fungible::Mutate as _;
        // Force set Alice's free balance on Para A
        pallet_balances::Pallet::<para_a::Runtime>::force_set_balance(
            para_a::RuntimeOrigin::root(),
            alice.clone().into(),
            amount * 10, // give her more than she'll send
        )
        .expect("force_set_balance ok");
    });

    // Show initial balances
    let before_a = TransparentP::execute_with(|| free_balance_a(&alice));
    let before_b = EncryptedP::execute_with(|| free_balance_b(&bob));

    println!("== Before ==");
    println!("Para A - Alice: {}", before_a);
    println!("Para B - Bob  : {}", before_b);

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
    let assets: Assets = (Here, amount).into();
    let fee_asset_item: u32 = 0;
    let weight_limit = WeightLimit::Unlimited;

    // 4) Dispatch the XCM from Para A (signed by Alice)
    TransparentP::execute_with(|| {
        let v_dest = xcm::VersionedLocation::from(dest.clone());
        let v_beneficiary = xcm::VersionedLocation::from(beneficiary.clone());
        let v_assets = xcm::VersionedAssets::from(assets.clone());
        let origin = para_a::RuntimeOrigin::signed(alice.clone());
        assert_ok!(para_a::PolkadotXcm::limited_reserve_transfer_assets(
            origin,
            bx!(v_dest),
            bx!(v_beneficiary),
            bx!(v_assets),
            fee_asset_item,
            weight_limit,
        ));
    });

    // Process HRMP/UMP/XCMP once we're OUT of the AssetHub externalities.
    // Any chain's `execute_with` will pump the queues; the relay is standard.
    EncryptedP::assert_xcmp_queue_success(None);

    // 5) One `execute_with` on a parachain triggers HRMP/UMP routing in the emulator.
    //    By here, messages are processed; assert/print final balances.

    let after_a = TransparentP::execute_with(|| free_balance_a(&alice));
    let after_b = EncryptedP::execute_with(|| free_balance_b(&bob));

    println!("\n== After ==");
    println!("Para A - Alice: {}", after_a);
    println!("Para B - Bob  : {}", after_b);

    // Pretty delta print
    println!("\n== Delta ==");
    println!("Para A - sent : {}", before_a.saturating_sub(after_a));
    println!("Para B - recv : {}", after_b.saturating_sub(before_b));

    // Optional assert that Bob received (within simple threshold)
    if after_b <= before_b {
        panic!("Bob did not receive funds on Para B");
    }
    info!("Reserve transfer complete.");
}
