use crate::pallet as pallet_confidential_assets;
use confidential_assets_primitives::{
    ConfidentialBackend, EncryptedAmount, InputProof, PublicKeyBytes, Ramp, ZkVerifier,
};
use frame_support::{construct_runtime, derive_impl};
use sp_runtime::BuildStorage;

pub type AccountId = u64;
pub type AssetId = u32;
pub type Balance = u64;
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const ASSET: AssetId = 7;

// --- A very simple, always-OK mock verifier ---------------------------------
// It returns deterministic 32-byte commitments and 64-byte ciphertexts.
// This allows us to assert pallet state transitions without touching ZK logic.

#[derive(Default)]
pub struct AlwaysOkVerifier;

impl ZkVerifier for AlwaysOkVerifier {
    type Error = ();
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

pub struct NoRamp;
impl Ramp<AccountId, AssetId, Balance> for NoRamp {
    type Error = ();

    fn transfer_from(
        _from: &AccountId,
        _to: &AccountId,
        _asset: AssetId,
        _amount: Balance,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn burn(_from: &AccountId, _asset: &AssetId, _amount: Balance) -> Result<(), Self::Error> {
        Ok(())
    }
    fn mint(_to: &AccountId, _asset: &AssetId, _amount: Balance) -> Result<(), Self::Error> {
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
impl pallet_confidential_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type Balance = Balance;
    type Backend = Zkhe;
    type Ramp = NoRamp;
    type AssetMetadata = ();
    type Acl = ();
    type Operators = ();
    type WeightInfo = ();
}

construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Zkhe: pallet_zkhe,
        ConfidentialAssets: pallet_confidential_assets,
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

// Accept envelope encoding: u16 count || ids (u64 LE) * count || rest (opaque)
pub fn accept_input(ids: &[u64], rest: &[u8]) -> InputProof {
    let mut v = Vec::with_capacity(2 + ids.len() * 8 + rest.len());
    let count = ids.len() as u16;
    v.extend_from_slice(&count.to_le_bytes());
    for id in ids {
        v.extend_from_slice(&id.to_le_bytes());
    }
    v.extend_from_slice(rest);
    proof(&v)
}
