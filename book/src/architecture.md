# Architecture Overview

Understanding the confidential assets system architecture.

## System Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    Client Application                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ User Wallet │  │ Key Mgmt    │  │ zkhe-prover (std)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Extrinsics + Proofs
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Substrate Runtime                         │
│  ┌─────────────────────────────────────────────────────┐    │
│  │           pallet-confidential-assets                 │    │
│  │  (ERC-7984 interface: deposit, withdraw, transfer)   │    │
│  └───────────────────────┬─────────────────────────────┘    │
│                          │ Backend trait                     │
│  ┌───────────────────────▼─────────────────────────────┐    │
│  │                  pallet-zkhe                         │    │
│  │  (Encrypted balance storage, UTXO management)        │    │
│  └───────────────────────┬─────────────────────────────┘    │
│                          │ Verifier trait                    │
│  ┌───────────────────────▼─────────────────────────────┐    │
│  │           zkhe-verifier (no_std)                     │    │
│  │  (ZK proof verification, range checks)               │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### pallet-confidential-assets

The **public interface** implementing ERC-7984:

```rust
pub trait Config: frame_system::Config {
    type AssetId;           // Asset identifier type
    type Balance;           // Balance value type
    type Backend;           // Cryptographic backend (pallet-zkhe)
    type Ramp;              // Public↔Confidential bridge
    type Acl;               // Access control (optional)
    type Operators;         // Operator permissions (optional)
    type AssetMetadata;     // Asset metadata provider (optional)
}
```

**Extrinsics:**
- `set_public_key(pk)` - Register encryption public key
- `deposit(asset, amount, proof)` - Convert public → confidential
- `withdraw(asset, encrypted_amount, proof)` - Convert confidential → public
- `confidential_transfer(asset, to, encrypted_delta, proof)` - Transfer
- `confidential_claim(asset, accept_envelope)` - Claim pending transfers
- `disclose_amount(asset, encrypted_amount)` - Reveal amount (owner only)

### pallet-zkhe

The **cryptographic backend** storing encrypted state:

**Storage:**
```rust
PublicKeys: Map<AccountId, PublicKeyBytes>
AvailableBalanceCommit: Map<(AssetId, AccountId), Commitment>
PendingBalanceCommit: Map<(AssetId, AccountId), Commitment>
TotalSupplyCommit: Map<AssetId, Commitment>
PendingDeposits: Map<(AccountId, AssetId, DepositId), EncryptedAmount>
```

**State Model:**
- **Available balance**: Spendable funds (Pedersen commitment)
- **Pending balance**: Incoming transfers awaiting claim
- **UTXO list**: Individual encrypted amounts for each pending transfer

### zkhe-verifier

**On-chain proof verification** (no_std compatible):

```rust
pub trait ZkVerifier {
    fn verify_transfer_sent(...) -> Result<(Commitment, Commitment), Error>;
    fn verify_transfer_received(...) -> Result<(Commitment, Commitment), Error>;
    fn verify_mint(...) -> Result<(Commitment, Commitment, EncryptedAmount), Error>;
    fn verify_burn(...) -> Result<(Commitment, Commitment, u64), Error>;
    fn disclose(...) -> Result<u64, Error>;
}
```

**Cryptographic operations:**
1. Parse and validate proof bundles
2. Verify Bulletproof range proofs (0 ≤ amount < 2^64)
3. Verify link proofs (commitment consistency)
4. Compute new balance commitments

### zkhe-prover

**Client-side proof generation** (std only):

```rust
// Transfer: sender generates proof
prove_sender_transfer(&SenderInput) -> Result<SenderOutput, ProverError>

// Receive: recipient accepts pending
prove_receiver_accept(&ReceiverAcceptInput) -> Result<ReceiverOutput, ProverError>

// Deposit: user mints confidential from public
prove_mint(&MintInput) -> Result<MintOutput, ProverError>

// Withdraw: user burns confidential to public
prove_burn(&BurnInput) -> Result<BurnOutput, ProverError>
```

## Data Flow: Confidential Transfer

```
1. SETUP: Both parties register public keys
   Sender:   set_public_key(pk_sender)
   Receiver: set_public_key(pk_receiver)

2. SEND: Sender creates and submits transfer
   ┌──────────────────────────────────────────────────────┐
   │ Client (Sender)                                      │
   │                                                      │
   │ prove_sender_transfer() generates:                   │
   │   - Δct: ElGamal ciphertext of amount               │
   │   - ΔC: Pedersen commitment to amount               │
   │   - Range proof: 0 ≤ amount < 2^64                  │
   │   - Link proof: Δct encrypts same value as ΔC      │
   │   - Balance proof: sender has sufficient funds      │
   └──────────────────────────────────────────────────────┘
                              │
                              ▼
   ┌──────────────────────────────────────────────────────┐
   │ On-chain                                             │
   │                                                      │
   │ verify_transfer_sent() checks:                       │
   │   - All proofs valid                                 │
   │   - Sender's new balance ≥ 0                        │
   │                                                      │
   │ Storage updates:                                     │
   │   - Sender: available -= ΔC                         │
   │   - Receiver: pending += ΔC                         │
   │   - UTXO created for receiver                       │
   └──────────────────────────────────────────────────────┘

3. RECEIVE: Receiver claims pending amount
   ┌──────────────────────────────────────────────────────┐
   │ Client (Receiver)                                    │
   │                                                      │
   │ prove_receiver_accept() generates:                   │
   │   - Decryption proof: knows sk to decrypt Δct       │
   │   - Balance proof: pending → available transfer     │
   └──────────────────────────────────────────────────────┘
                              │
                              ▼
   ┌──────────────────────────────────────────────────────┐
   │ On-chain                                             │
   │                                                      │
   │ verify_transfer_received() checks:                   │
   │   - Receiver knows secret key                       │
   │   - Balance update is correct                       │
   │                                                      │
   │ Storage updates:                                     │
   │   - Receiver: available += ΔC, pending -= ΔC        │
   │   - UTXOs consumed                                  │
   └──────────────────────────────────────────────────────┘
```

## Commitment Scheme

All balances are stored as **Pedersen commitments**:

```
C = v·G + r·H

Where:
  v = plaintext value (secret)
  r = randomness (secret)
  G = generator point (public)
  H = hash-derived generator (public)
  C = commitment (public, stored on-chain)
```

**Properties:**
- **Hiding**: Cannot determine v from C without r
- **Binding**: Cannot find different (v', r') producing same C
- **Homomorphic**: C1 + C2 = (v1+v2)·G + (r1+r2)·H

## Encryption Scheme

Amounts are encrypted using **twisted ElGamal**:

```
Ciphertext(v) = (C, D) where:
  C = v·G + r·H     (Pedersen commitment)
  D = r·pk          (encryption key share)

Decryption with sk:
  v·G = C - sk·D/sk = C - r·G
  (Then brute-force v from v·G)
```

## Security Model

| Property | Guarantee |
|----------|-----------|
| Amount privacy | Only sender/receiver know transfer amounts |
| Address transparency | All addresses are public |
| Balance integrity | ZK proofs ensure balance ≥ 0 |
| Supply conservation | Sum of all balances = total supply |
| No double-spend | UTXO consumption is atomic |

## Cross-Chain Architecture

For XCM confidential transfers, see [XCM Setup](./xcm-setup.md).

```
ParaA                           ParaB
┌─────────────┐                ┌─────────────┐
│ Sender      │                │ Receiver    │
│             │                │             │
│ escrow(Δ)   │───── XCM ─────▶│ mint(Δ)     │
│             │◀──── XCM ──────│ confirm()   │
│ release()   │                │             │
└─────────────┘                └─────────────┘
```
