//! xcm-emulator functionality required to implement manually because it is INCOMPLETE AF
use core::marker::PhantomData;
use frame_support::{
    dispatch::{DispatchResult, GetDispatchInfo, RawOrigin},
    inherent::{InherentData, ProvideInherent},
    pallet_prelude::Get,
    traits::{OnFinalize, OnInitialize, OriginTrait, UnfilteredDispatchable},
    weights::Weight,
};
use frame_system::pallet_prelude::{BlockNumberFor, HeaderFor};
use futures::executor::block_on;
use polkadot_parachain_primitives::primitives::{
    HeadData, HrmpChannelId, RelayChainBlockNumber, XcmpMessageFormat,
};
use polkadot_runtime_parachains::paras_inherent;
use polkadot_sdk::{staging_xcm as xcm, staging_xcm_builder as xcm_builder, *};
use sp_consensus_aura::{SlotDuration, AURA_ENGINE_ID};
use sp_core::{Encode, U256};
use sp_runtime::traits::Header as _;
use sp_runtime::{
    traits::{Dispatchable, Header},
    BuildStorage, Digest, DigestItem, DispatchError, Either, SaturatedConversion,
};

pub struct RuntimeHelper<Runtime, AllPalletsWithoutSystem>(
    PhantomData<(Runtime, AllPalletsWithoutSystem)>,
);
/// Utility function that advances the chain to the desired block number.
impl<
        Runtime: frame_system::Config
            + pallet_timestamp::Config
            + polkadot_runtime_parachains::paras_inherent::Config,
        AllPalletsWithoutSystem: OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
    > RuntimeHelper<Runtime, AllPalletsWithoutSystem>
{
    pub fn run_to_block(n: u32) {
        let mut last_header = None;
        loop {
            let block_number = frame_system::Pallet::<Runtime>::block_number();
            if block_number >= n.into() {
                break;
            }
            println!("Block: {block_number}");
            // finalize current block
            let header = Self::finalize_current_block(block_number);
            // reset events and start next block
            frame_system::Pallet::<Runtime>::reset_events();
            let next_block_number = block_number + 1u32.into();
            frame_system::Pallet::<Runtime>::initialize(
                &next_block_number,
                &header.hash(),
                &Digest { logs: vec![] }, //could insert block author here by adding input arg
            );
            AllPalletsWithoutSystem::on_initialize(next_block_number);
            last_header = Some(header);
        }
    }
    pub fn root_origin() -> <Runtime as frame_system::Config>::RuntimeOrigin {
        <Runtime as frame_system::Config>::RuntimeOrigin::root()
    }
    pub fn block_number() -> BlockNumberFor<Runtime> {
        frame_system::Pallet::<Runtime>::block_number()
    }
    // Finalize current block
    pub fn finalize_current_block(block_number: BlockNumberFor<Runtime>) -> HeaderFor<Runtime> {
        use sp_inherents::InherentDataProvider;
        // 1) Build inherent data bag
        let mut inherent = InherentData::new();

        // a) timestamp
        {
            let ts = sp_timestamp::InherentDataProvider::new(300u64.into());
            block_on(ts.provide_inherent_data(&mut inherent)).expect("timestamp inherent");
            let _ = pallet_timestamp::Pallet::<Runtime>::set(
                <Runtime as frame_system::Config>::RuntimeOrigin::none(),
                300u32.into(),
            );
        }

        // b) paras_inherent THIS DOESNT FUCKING WORK
        // {
        //     use polkadot_runtime_parachains::paras_inherent as ph;
        //     let ts = cumulus_client_parachain_inherent::MockValidationDataInherentDataProvider::<()>::default();
        //     block_on(ts.provide_inherent_data(&mut inherent)).expect("paras_inherent data");
        //     let maybe_inherent = <cumulus_pallet_parachain_system::Pallet::<Runtime> as ProvideInherent>::create_inherent(&inherent);
        //     if let Some(cumulus_pallet_parachain_system::Call::<Runtime>::enter { data }) = maybe_inherent {
        //         let result = ph::Pallet::<Runtime>::enter(
        //             <Runtime as frame_system::Config>::RuntimeOrigin::none(),
        //             data,
        //         );
        //         if result.is_err() {
        //             println!("parachain inherent not parsed and not submitted");
        //         }
        //     }
        // }

        AllPalletsWithoutSystem::on_finalize(block_number);
        frame_system::Pallet::<Runtime>::finalize()
    }
}

//else {
//         use cumulus_primitives_parachain_inherent::{
//             ParachainInherentData, PersistedValidationData,
//         };
//         let data = ParachainInherentData {
//             validation_data: PersistedValidationData {
//                 relay_parent_number: block_number,
//                 ..Default::default()
//             },
//             downward_messages: todo!(),
// horizontal_messages: todo!(),
// relay_chain_state: todo!(),
//         };
//         let result = ph::Pallet::<Runtime>::enter(
//             <Runtime as frame_system::Config>::RuntimeOrigin::none(),
//             data,
//         );
//         if result.is_err() {
//             println!("parachain inherent not parsed and not submitted");
//         }
//     }
