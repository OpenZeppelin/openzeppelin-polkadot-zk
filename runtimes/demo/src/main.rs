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
use polkadot_primitives::{ValidationCode, MIN_CODE_SIZE};
use polkadot_runtime_common::paras_registrar;
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
    para_a::RuntimeGenesisConfig {
        parachain_info: para_a::ParachainInfoConfig {
            parachain_id: 100.into(),
            ..Default::default()
        },
        ..Default::default()
    }
    .build_storage()
    .expect("para A genesis storage")
}

fn para_b_genesis() -> sp_core::storage::Storage {
    para_b::RuntimeGenesisConfig {
        parachain_info: para_b::ParachainInfoConfig {
            parachain_id: 200.into(),
            ..Default::default()
        },
        ..Default::default()
    }
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
            Paras: relay::Paras,
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

// No wonder nobody uses Polkadot its an absolute PoS framework drowning in complexity
fn register_paras_then_open_hrmp() {
    Relay::execute_with(|| {
        println!("-- 1. Register Parachains");
        assert_ok!(relay::Registrar::force_register(
            relay::RuntimeOrigin::root(),
            id("Alice"), //owner
            0,
            EncryptedP::para_id(),
            Default::default(),
            ValidationCode(vec![0u8; MIN_CODE_SIZE.try_into().unwrap()]),
        ));
        assert_ok!(relay::Registrar::force_register(
            relay::RuntimeOrigin::root(),
            id("Alice"), //owner
            0,
            TransparentP::para_id(),
            Default::default(),
            ValidationCode(vec![0u8; MIN_CODE_SIZE.try_into().unwrap()]),
        ));
        println!("-- parachains successfully force registered");

        println!("-- 2. Onboard Parachains");
        // TODO: everything needed for onboarding holy actual fuck
        let e_state = relay::Paras::lifecycle(EncryptedP::para_id())
            .expect("no parachain state for ConfidentialHub");
        assert!(!e_state.is_onboarding(), "ConfidentialHub is onboarding");
        println!("-- parachains successfully onboarded");

        println!("-- 3. Open HRMP channels A<->B");
        assert_ok!(relay::Hrmp::force_open_hrmp_channel(
            relay::RuntimeOrigin::root(),
            EncryptedP::para_id(),
            TransparentP::para_id(),
            1_000_000_000,
            1024,
        ));
        assert_ok!(relay::Hrmp::force_open_hrmp_channel(
            relay::RuntimeOrigin::root(),
            TransparentP::para_id(),
            EncryptedP::para_id(),
            1_000_000_000,
            1024,
        ));
        println!("-- HRMP channels forced open A<->B");
    });
}

fn print_sovereign_of_b_on_a() {
    TransparentP::execute_with(|| {
        let loc = Location::new(1, [Junction::Parachain(EncryptedP::para_id().into())]);
        let who = <para_a::configs::LocationToAccountId as crate::ConvertLocation<
            para_a::AccountId,
        >>::convert_location(&loc)
        .expect("LocationToAccountId conversion failed");
        let free = para_a::Balances::free_balance(&who);
        println!("On A: sovereign-of-B account = {who:?} free={free}");
    });
}

fn pump_until_idle(label: &str, rounds: usize) {
    for r in 0..rounds {
        println!("-- pump {label} round {r}");

        // drain B's inbound/outbound XCMP and assert processed (if any)
        EncryptedP::execute_with(|| {
            // This uses your impl_assert_events_helpers_for_parachain!
            EncryptedP::assert_xcmp_queue_success(None);
        });

        // drain A's inbound/outbound XCMP and assert processed (if any)
        TransparentP::execute_with(|| {
            TransparentP::assert_xcmp_queue_success(None);
        });

        // Tick relay (DMP/MQ movement)
        Relay::execute_with(|| {});

        // Your existing prints (System events) to see any Remarked, etc.
        print_events_a(&format!("after-pump-{r}"));
        print_events_b(&format!("after-pump-{r}"));
    }
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

    // Give a generous budget first; we’ll tune later from events.
    let required = Weight::from_parts(2_000_000_000, 0);

    let msg = Xcm(vec![Transact {
        origin_kind: OriginKind::SovereignAccount,
        fallback_max_weight: Some(required),
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
        // This comes from impl_assert_events_helpers_for_parachain!
        EncryptedP::assert_xcm_pallet_sent(); // Pallet_xcm::Sent{..}
                                              // Also assert we at least "Attempted" on origin pallet
                                              // (Barrier on origin may emit Attempted::Complete/Incomplete/Error)
                                              // We don't know expected weight yet; pass None.
        EncryptedP::assert_xcm_pallet_attempted_complete(None);
        // If you expect it to be incomplete instead, comment the above and use:
        // EncryptedP::assert_xcm_pallet_attempted_incomplete(None, None);
    });
}

fn main() {
    // (Optional) set RUST_LOG=info to see emulator event logs.
    // env_logger::init(); // If you add env_logger to Cargo.toml

    let alice_b = para_b::RuntimeOrigin::signed(id("Alice"));

    print_events_a("before");
    print_events_b("before");

    register_paras_then_open_hrmp();
    print_sovereign_of_b_on_a();

    // Fire Transact B -> A (unpaid first; we’ll learn what A’s Barrier says)
    send_transact_from_b_to_a(alice_b);

    // Move messages; assert processed via your MQ assertions
    pump_until_idle("B->A transact", 5);

    print_events_a("after");
    print_events_b("after");

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
        if saw_remark {
            println!("FAILED: Transact did not execute System.remark* on Para A");
        } else {
            println!("FAILED: Transact did not execute System.remark* on Para A");
        }
    });
}
