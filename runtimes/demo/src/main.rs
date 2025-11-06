//! XCM Integration Tests for Cross Chain Confidential Transfers
//!
//! Tests include:
//! - XCM reserve transfer of plaintext asset from AssetHub (Asset Hub) -> ConfidentialHub (Confidential Hub)

use asset_hub_runtime as para_a;
use confidential_runtime as para_b;
use emulated_integration_tests_common::{
    impl_accounts_helpers_for_parachain, impl_accounts_helpers_for_relay_chain,
    impl_assert_events_helpers_for_parachain, impl_assert_events_helpers_for_relay_chain,
    impl_hrmp_channels_helpers_for_relay_chain, impl_send_transact_helpers_for_relay_chain,
    impl_xcm_helpers_for_parachain, impls::Parachain,
};
use frame_support::assert_ok;
use log::info;
use polkadot_sdk::{staging_xcm as xcm, staging_xcm_builder as xcm_builder, *};
use relay_runtime as relay;
use relay_runtime::BuildStorage;
use xcm_builder::test_utils::*;
use xcm_emulator::*;

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

fn print_events_a(tag: &str) {
    TransparentP::execute_with(|| {
        println!("--- events on AssetHub ({tag}) ---");
        for e in para_a::System::events() {
            println!("{:?}", e.event);
        }
    });
}
fn print_events_b(tag: &str) {
    EncryptedP::execute_with(|| {
        println!("--- events on ConfidentialHub ({tag}) ---");
        for e in para_b::System::events() {
            println!("{:?}", e.event);
        }
    });
}

fn build_call_for_a() -> para_a::RuntimeCall {
    // Use a call that requires no balance on the sovereign account and no special origin.
    // Newer SDKs: `remark_with_event(Vec<u8>)`
    para_a::RuntimeCall::System(frame_system::Call::<para_a::Runtime>::remark_with_event {
        remark: b"hello-from-B-via-Transact".to_vec(),
    })
    // If your System pallet doesn't have `remark_with_event`, use:
    // para_a::RuntimeCall::System(frame_system::Call::<para_a::Runtime>::remark { remark: b"...".to_vec() })
}

// ---------- Send Transact from B -> A ----------
fn send_transact_from_b_to_a(origin_signed_on_b: para_b::RuntimeOrigin) {
    // Destination: Parent, Parachain(A)
    let dest_to_a: Location = EncryptedP::sibling_location_of(TransparentP::para_id());
    println!("dest_to_a = {:?}", dest_to_a);

    // Build the call that will execute on A.
    let call_on_a: para_a::RuntimeCall = build_call_for_a();
    // Wrap it for XCM Transact
    let call_data = call_on_a.encode();
    let xcm_call = call_data.into();

    // Give a generous weight budget for the call execution on A.
    // (Tune down once you see actual weight in events.)
    let required = Weight::from_parts(2_000_000_000, 0);

    // Compose XCM message: just Transact with SovereignAccount origin.
    let msg = Xcm(vec![Transact {
        origin_kind: OriginKind::SovereignAccount,
        fallback_max_weight: None,
        call: xcm_call,
    }]);

    // Send from B
    EncryptedP::execute_with(|| {
        println!("-- sending XCM Transact from B to A");
        let v_dest = xcm::VersionedLocation::from(dest_to_a.clone());
        let v_xcm = xcm::VersionedXcm::from(msg.clone());

        // Most parachains allow Signed to send XCM; if not, switch to Root for this test.
        assert_ok!(para_b::PolkadotXcm::send(
            origin_signed_on_b, // e.g., RuntimeOrigin::signed(Alice)
            bx!(v_dest),
            bx!(v_xcm),
        ));
    });
}

fn main() {
    // (Optional) set RUST_LOG=info to see emulator event logs.
    // env_logger::init(); // If you add env_logger to Cargo.toml

    let alice_b_signed = para_b::RuntimeOrigin::signed(id("Alice"));

    // Pre-print events (should be empty-ish)
    print_events_a("before");
    print_events_b("before");

    // LocalRelay::execute_with(|| {
    //     // open both directions in the emulator (ids: change if your paras differ)
    //     relay::Hrmp::force_open_hrmp_channel(ConfidentialHub::para_id(), AssetHub::para_id(), 1_000_000_000, 1024);
    //     relay::Hrmp::force_open_hrmp_channel(AssetHub::para_id(), ConfidentialHub::para_id(), 1_000_000_000, 1024);
    // });

    // 1) Fire a pure Transact from B -> A
    send_transact_from_b_to_a(alice_b_signed);

    // Pump queues from *either* para (externalities required)
    EncryptedP::execute_with(|| {
        // Flushing B’s outbound / inbound HRMP queues
        EncryptedP::assert_xcmp_queue_success(None);
    });

    // Assert success on the DESTINATION chain (A)
    TransparentP::execute_with(|| {
        // If you’ve got the helper, use it; otherwise just print events.
        // Prefer this over asserting on B.
        TransparentP::assert_xcmp_queue_success(None);
    });

    // 3) Print events again to confirm it ran on A
    print_events_a("after");
    print_events_b("after");

    // 4) Minimal assertion: look for System::Remarked (or pallet index/variant) on A
    TransparentP::execute_with(|| {
        let mut saw_remark = false;
        for e in para_a::System::events() {
            // Match on your SDK's event enum
            if let para_a::RuntimeEvent::System(frame_system::Event::Remarked { .. }) = e.event {
                saw_remark = true;
                break;
            }
            // If using `remark_with_event`, event is also `System::Remarked`
        }
        assert!(
            saw_remark,
            "Transact did not execute System.remark* on Para A"
        );
    });

    println!("OK: Transact from B → A executed successfully.");
}
