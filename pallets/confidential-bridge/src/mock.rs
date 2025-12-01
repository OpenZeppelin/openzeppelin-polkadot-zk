use crate::pallet as pallet_confidential_bridge;
use confidential_assets_primitives::{
    ConfidentialBackend, EncryptedAmount, HrmpMessenger, InputProof, NetworkIdProvider,
    PublicKeyBytes, ZkVerifier,
};
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
    PalletId,
};
use sp_runtime::BuildStorage;

pub type AccountId = u64;
pub type AssetId = u32;
pub type Balance = u64;
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const ASSET: AssetId = 7;

// --- Mock Network ID Provider -----------------------------------------------
pub struct MockNetworkId;
impl NetworkIdProvider for MockNetworkId {
    fn network_id() -> [u8; 32] {
        [0u8; 32]
    }
}

// --- A very simple, always-OK mock verifier ---------------------------------
// It returns deterministic 32-byte commitments and 64-byte ciphertexts.
// This allows us to assert pallet state transitions without touching ZK logic.

#[derive(Default)]
pub struct AlwaysOkVerifier;

impl ZkVerifier for AlwaysOkVerifier {
    type Error = ();
    type NetworkIdProvider = MockNetworkId;
    // Disclose encrypted amount -> constant u64 (e.g., 123)
    fn disclose(_asset: &[u8], _pk: &[u8], _cipher: &[u8]) -> Result<u64, ()> {
        Ok(123)
    }

    // from_new_available, to_new_pending
    fn verify_transfer_sent(
        _asset: &[u8],
        _from_pk: &[u8],
        _to_pk: &[u8],
        _from_old_avail: &[u8],
        _to_old_pending: &[u8],
        _delta_ct: &[u8],
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), ()> {
        Ok((vec![1u8; 32], vec![2u8; 32]))
    }

    // avail_new, pending_new
    fn verify_transfer_received(
        _asset: &[u8],
        _who_pk: &[u8],
        _avail_old: &[u8],
        _pending_old: &[u8],
        _commits: &[[u8; 32]],
        _envelope: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), ()> {
        // Make pending_new zero so pallet removes PendingBalanceCommit on accept
        Ok((vec![3u8; 32], vec![0u8; 32]))
    }

    // to_new_pending, total_new, minted_ct
    fn verify_mint(
        _asset: &[u8],
        _to_pk: &PublicKeyBytes,
        _to_old_pending: &[u8],
        _total_old: &[u8],
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, EncryptedAmount), ()> {
        Ok((vec![10u8; 32], vec![11u8; 32], [5u8; 64]))
    }

    // from_new_available, total_new, disclosed_u64
    fn verify_burn(
        _asset: &[u8],
        _from_pk: &PublicKeyBytes,
        _from_old_avail: &[u8],
        _total_old: &[u8],
        _amount_ct: &EncryptedAmount,
        _proof: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>, u64), ()> {
        Ok((vec![20u8; 32], vec![21u8; 32], 42))
    }
}

pub struct MockMessenger;
impl HrmpMessenger for MockMessenger {
    /// Send an opaque SCALE-encoded payload to `dest_para`.
    fn send(_dest_para: u32, _payload: Vec<u8>) -> Result<(), ()> {
        Ok(())
    }
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = frame_system::mocking::MockBlock<Runtime>;
}

impl pallet_zkhe::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Verifier = AlwaysOkVerifier;
    type WeightInfo = ();
}
parameter_types! {
    pub const EscrowPalletId: PalletId = PalletId(*b"CaEscrow");
    pub const BridgePalletId: PalletId = PalletId(*b"CaBridge");
}
impl pallet_confidential_escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type PalletId = EscrowPalletId;
}
impl pallet_confidential_bridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type Escrow = ConfidentialEscrow;
    type Messenger = MockMessenger;
    type MaxBridgePayload = ConstU32<1024>;
    type BurnPalletId = BridgePalletId;
    type DefaultTimeout = ConstU64<10>;
    type SelfParaId = ConstU32<1>;
    type XcmOrigin = frame_system::EnsureRoot<AccountId>;
    type WeightInfo = ();
}

construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Zkhe: pallet_zkhe,
        ConfidentialEscrow: pallet_confidential_escrow,
        ConfidentialBridge: pallet_confidential_bridge,
    }
);

// Build a fresh externalities for each test.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();
    // nothing else needed in genesis
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// Handy helpers
pub fn set_pk(who: AccountId) {
    // Non-empty fake pk
    Zkhe::set_public_key(&who, &[7u8; 64].to_vec().try_into().expect("bounded vec")).unwrap();
}

// Construct InputProof from raw bytes using TryFrom<Vec<u8>>
pub fn proof(bytes: &[u8]) -> InputProof {
    bytes.to_vec().try_into().expect("bounded vec")
}
