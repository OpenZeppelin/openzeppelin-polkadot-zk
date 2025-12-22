polkadot_sdk::frame_benchmarking::define_benchmarks!(
    [frame_system, SystemBench::<Runtime>]
    [pallet_balances, Balances]
    [pallet_session, SessionBench::<Runtime>]
    [pallet_timestamp, Timestamp]
    [pallet_message_queue, MessageQueue]
    [pallet_sudo, Sudo]
    [pallet_collator_selection, CollatorSelection]
    [cumulus_pallet_parachain_system, ParachainSystem]
    [cumulus_pallet_xcmp_queue, XcmpQueue]
    [cumulus_pallet_weight_reclaim, WeightReclaim]
    // Confidential pallets
    [pallet_zkhe, Zkhe]
    [pallet_confidential_assets, ConfidentialAssets]
);
