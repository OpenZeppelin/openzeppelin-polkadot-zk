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

**Extrinsics:**
- `set_public_key(pk)` - Register encryption public key
- `deposit(asset, amount, proof)` - Convert public to confidential
- `withdraw(asset, encrypted_amount, proof)` - Convert confidential to public
- `confidential_transfer(asset, to, encrypted_delta, proof)` - Transfer
- `confidential_claim(asset, accept_envelope)` - Claim pending transfers
- `disclose_amount(asset, encrypted_amount)` - Reveal amount (owner only)

### pallet-zkhe

The **cryptographic backend** storing encrypted state:

**Storage:**
- `PublicKeys` - Account to public key mapping
- `AvailableBalanceCommit` - Spendable balance commitments
- `PendingBalanceCommit` - Incoming transfer commitments
- `TotalSupplyCommit` - Asset total supply commitment
- `PendingDeposits` - Individual encrypted pending amounts (UTXOs)

**State Model:**
- **Available balance**: Spendable funds (Pedersen commitment)
- **Pending balance**: Incoming transfers awaiting claim
- **UTXO list**: Individual encrypted amounts for each pending transfer

### zkhe-verifier

**On-chain proof verification** (no_std compatible):

- `verify_transfer_sent` - Validates sender's transfer proof
- `verify_transfer_received` - Validates receiver's acceptance proof
- `verify_mint` - Validates deposit/mint proof
- `verify_burn` - Validates withdrawal/burn proof
- `disclose` - Decrypts amount for authorized disclosure

### zkhe-prover

**Client-side proof generation** (std only):

- `prove_sender_transfer` - Generate sender's transfer proof
- `prove_receiver_accept` - Generate receiver's acceptance proof
- `prove_mint` - Generate deposit proof
- `prove_burn` - Generate withdrawal proof

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
   │   - Encrypted transfer amount                        │
   │   - Balance commitment update                        │
   │   - Range proof (amount ≥ 0)                        │
   │   - Link proof (encryption matches commitment)       │
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
   │   - Sender: available balance reduced                │
   │   - Receiver: pending balance increased              │
   │   - UTXO created for receiver                       │
   └──────────────────────────────────────────────────────┘

3. RECEIVE: Receiver claims pending amount
   ┌──────────────────────────────────────────────────────┐
   │ Client (Receiver)                                    │
   │                                                      │
   │ prove_receiver_accept() generates:                   │
   │   - Decryption proof (knows secret key)              │
   │   - Balance update proof                             │
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
   │   - Receiver: pending → available                   │
   │   - UTXOs consumed                                  │
   └──────────────────────────────────────────────────────┘
```

## Cryptography

All balances are stored as Pedersen commitments, and amounts are encrypted using twisted ElGamal. See [Cryptographic Primitives](./crypto.md) for details.

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
┌─────────────────┐            ┌─────────────────┐
│ Sender          │            │ Receiver        │
│                 │            │                 │
│ escrow(amount)  │─── XCM ───▶│ mint(amount)    │
│                 │◀── XCM ────│ confirm()       │
│ release()       │            │                 │
└─────────────────┘            └─────────────────┘
```
