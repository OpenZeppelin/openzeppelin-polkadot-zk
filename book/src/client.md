# Client Integration

Build client applications that interact with confidential assets.

## Overview

Client applications must handle:

1. **Key Management**: Generate and store ElGamal keypairs
2. **Proof Generation**: Create ZK proofs for operations
3. **Balance Tracking**: Decrypt and track confidential balances
4. **Transaction Building**: Construct and submit extrinsics

## JavaScript/TypeScript SDK

### Installation

```bash
npm install @polkadot/api @polkadot/util-crypto
# Plus your WASM proof generator
npm install zkhe-prover-wasm
```

### API Connection

```typescript
import { ApiPromise, WsProvider } from '@polkadot/api';

const wsProvider = new WsProvider('wss://your-node.example.com');
const api = await ApiPromise.create({
  provider: wsProvider,
  types: {
    // Custom types for confidential assets
    EncryptedAmount: '[u8; 64]',
    Commitment: '[u8; 32]',
    PublicKeyBytes: '[u8; 32]',
  }
});
```

### Key Management

```typescript
import { randomBytes } from 'crypto';
import { zkheKeygen } from 'zkhe-prover-wasm';

// Generate ElGamal keypair
function generateConfidentialKeypair(): { secretKey: Uint8Array, publicKey: Uint8Array } {
  const seed = randomBytes(32);
  const { sk, pk } = zkheKeygen(seed);
  return { secretKey: sk, publicKey: pk };
}

// Store keys securely (example with localStorage - use secure storage in production)
function storeKeypair(accountId: string, keypair: { secretKey: Uint8Array, publicKey: Uint8Array }) {
  const encrypted = encryptWithPassword(keypair.secretKey, userPassword);
  localStorage.setItem(`confidential_sk_${accountId}`, encrypted);
  localStorage.setItem(`confidential_pk_${accountId}`, Buffer.from(keypair.publicKey).toString('hex'));
}

function loadPublicKey(accountId: string): Uint8Array | null {
  const hex = localStorage.getItem(`confidential_pk_${accountId}`);
  return hex ? Buffer.from(hex, 'hex') : null;
}
```

### Register Public Key

```typescript
async function registerPublicKey(api: ApiPromise, signer: KeyringPair, publicKey: Uint8Array) {
  const tx = api.tx.confidentialAssets.setPublicKey(publicKey);

  const hash = await tx.signAndSend(signer, { nonce: -1 });
  console.log('Public key registered:', hash.toHex());

  return hash;
}
```

### Query Balances

```typescript
// Get encrypted balance commitment
async function getBalanceCommitment(
  api: ApiPromise,
  assetId: number,
  account: string
): Promise<Uint8Array> {
  const result = await api.rpc.state.call(
    'ConfidentialAssetsApi_balance_of',
    api.createType('(u128, AccountId32)', [assetId, account]).toHex()
  );
  return result.toU8a();
}

// Get pending balance commitment
async function getPendingBalance(
  api: ApiPromise,
  assetId: number,
  account: string
): Promise<Uint8Array | null> {
  const result = await api.query.zkhe.pendingBalanceCommit(assetId, account);
  return result.isSome ? result.unwrap().toU8a() : null;
}

// Decrypt balance (requires secret key)
function decryptBalance(
  encryptedCommit: Uint8Array,
  secretKey: Uint8Array,
  assetId: number
): bigint {
  // Use WASM prover to decrypt
  const { zkheDecrypt } = require('zkhe-prover-wasm');
  return zkheDecrypt(encryptedCommit, secretKey, assetId);
}
```

### Deposit (Public → Confidential)

```typescript
import { zkheProveMint } from 'zkhe-prover-wasm';

async function deposit(
  api: ApiPromise,
  signer: KeyringPair,
  assetId: number,
  amount: bigint,
  recipientPk: Uint8Array,
  currentPendingCommit: Uint8Array,
  currentTotalSupply: Uint8Array
) {
  // Generate mint proof
  const mintInput = {
    asset_id: assetId,
    recipient_pk: recipientPk,
    amount: amount,
    pending_old: currentPendingCommit,
    total_old: currentTotalSupply,
  };

  const proof = await zkheProveMint(mintInput);

  // Submit deposit transaction
  const tx = api.tx.confidentialAssets.deposit(assetId, amount, proof);

  return tx.signAndSend(signer, { nonce: -1 });
}
```

### Confidential Transfer

```typescript
import { zkheProveSenderTransfer } from 'zkhe-prover-wasm';

async function confidentialTransfer(
  api: ApiPromise,
  signer: KeyringPair,
  assetId: number,
  recipientAccount: string,
  amount: bigint,
  senderSk: Uint8Array,
  senderPk: Uint8Array,
  recipientPk: Uint8Array,
  senderAvailCommit: Uint8Array,
  recipientPendingCommit: Uint8Array
) {
  // Generate sender proof
  const transferInput = {
    asset_id: assetId,
    sender_sk: senderSk,
    sender_pk: senderPk,
    recipient_pk: recipientPk,
    amount: amount,
    sender_avail_old: senderAvailCommit,
    recipient_pending_old: recipientPendingCommit,
  };

  const { delta_ct, proof } = await zkheProveSenderTransfer(transferInput);

  // Submit transfer transaction
  const tx = api.tx.confidentialAssets.confidentialTransfer(
    assetId,
    recipientAccount,
    delta_ct,
    proof
  );

  return tx.signAndSend(signer, { nonce: -1 });
}
```

### Accept Pending Transfer

```typescript
import { zkheProveReceiverAccept } from 'zkhe-prover-wasm';

async function acceptPending(
  api: ApiPromise,
  signer: KeyringPair,
  assetId: number,
  recipientSk: Uint8Array,
  recipientPk: Uint8Array,
  availCommit: Uint8Array,
  pendingCommit: Uint8Array,
  utxoCommits: Uint8Array[]  // From pending UTXOs
) {
  // Generate accept proof
  const acceptInput = {
    asset_id: assetId,
    recipient_sk: recipientSk,
    recipient_pk: recipientPk,
    avail_old: availCommit,
    pending_old: pendingCommit,
    utxo_commits: utxoCommits,
  };

  const proof = await zkheProveReceiverAccept(acceptInput);

  // Submit accept transaction
  const tx = api.tx.confidentialAssets.acceptPending(assetId, proof);

  return tx.signAndSend(signer, { nonce: -1 });
}
```

### Withdraw (Confidential → Public)

```typescript
import { zkheProveBurn } from 'zkhe-prover-wasm';

async function withdraw(
  api: ApiPromise,
  signer: KeyringPair,
  assetId: number,
  amount: bigint,
  senderSk: Uint8Array,
  senderPk: Uint8Array,
  availCommit: Uint8Array,
  totalSupplyCommit: Uint8Array
) {
  // Generate burn proof
  const burnInput = {
    asset_id: assetId,
    sender_sk: senderSk,
    sender_pk: senderPk,
    amount: amount,
    avail_old: availCommit,
    total_old: totalSupplyCommit,
  };

  const { amount_ct, proof } = await zkheProveBurn(burnInput);

  // Submit withdraw transaction
  const tx = api.tx.confidentialAssets.withdraw(assetId, amount_ct, proof);

  return tx.signAndSend(signer, { nonce: -1 });
}
```

## React Integration

### Context Provider

```typescript
import React, { createContext, useContext, useState, useEffect } from 'react';
import { ApiPromise, WsProvider } from '@polkadot/api';

interface ConfidentialAssetsContextType {
  api: ApiPromise | null;
  connected: boolean;
  keypair: { sk: Uint8Array; pk: Uint8Array } | null;
  setKeypair: (kp: { sk: Uint8Array; pk: Uint8Array }) => void;
}

const ConfidentialAssetsContext = createContext<ConfidentialAssetsContextType | null>(null);

export function ConfidentialAssetsProvider({ children, wsEndpoint }: {
  children: React.ReactNode;
  wsEndpoint: string;
}) {
  const [api, setApi] = useState<ApiPromise | null>(null);
  const [connected, setConnected] = useState(false);
  const [keypair, setKeypair] = useState<{ sk: Uint8Array; pk: Uint8Array } | null>(null);

  useEffect(() => {
    async function connect() {
      const provider = new WsProvider(wsEndpoint);
      const apiInstance = await ApiPromise.create({ provider });
      setApi(apiInstance);
      setConnected(true);
    }
    connect();
  }, [wsEndpoint]);

  return (
    <ConfidentialAssetsContext.Provider value={{ api, connected, keypair, setKeypair }}>
      {children}
    </ConfidentialAssetsContext.Provider>
  );
}

export function useConfidentialAssets() {
  const context = useContext(ConfidentialAssetsContext);
  if (!context) throw new Error('Must be used within ConfidentialAssetsProvider');
  return context;
}
```

### Balance Display Component

```typescript
import { useState, useEffect } from 'react';
import { useConfidentialAssets } from './ConfidentialAssetsProvider';
import { zkheDecrypt } from 'zkhe-prover-wasm';

interface BalanceDisplayProps {
  assetId: number;
  account: string;
}

export function BalanceDisplay({ assetId, account }: BalanceDisplayProps) {
  const { api, keypair } = useConfidentialAssets();
  const [balance, setBalance] = useState<bigint | null>(null);
  const [pending, setPending] = useState<bigint | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!api || !keypair) return;

    async function fetchBalances() {
      setLoading(true);

      // Fetch commitments
      const availCommit = await api.rpc.state.call(
        'ConfidentialAssetsApi_balance_of',
        api.createType('(u128, AccountId32)', [assetId, account]).toHex()
      );

      const pendingCommit = await api.query.zkhe.pendingBalanceCommit(assetId, account);

      // Decrypt with secret key
      const decryptedAvail = zkheDecrypt(availCommit.toU8a(), keypair.sk, assetId);
      setBalance(decryptedAvail);

      if (pendingCommit.isSome) {
        const decryptedPending = zkheDecrypt(pendingCommit.unwrap().toU8a(), keypair.sk, assetId);
        setPending(decryptedPending);
      }

      setLoading(false);
    }

    fetchBalances();

    // Subscribe to updates
    const unsub = api.query.zkhe.availableBalanceCommit(assetId, account, () => {
      fetchBalances();
    });

    return () => { unsub.then(u => u()); };
  }, [api, keypair, assetId, account]);

  if (loading) return <div>Loading...</div>;

  return (
    <div>
      <p>Available: {balance?.toString() ?? '0'}</p>
      {pending && pending > 0n && (
        <p>Pending: {pending.toString()} (claim required)</p>
      )}
    </div>
  );
}
```

### Transfer Form Component

```typescript
import { useState } from 'react';
import { useConfidentialAssets } from './ConfidentialAssetsProvider';
import { web3FromAddress } from '@polkadot/extension-dapp';
import { zkheProveSenderTransfer } from 'zkhe-prover-wasm';

export function TransferForm({ assetId, sender }: { assetId: number; sender: string }) {
  const { api, keypair } = useConfidentialAssets();
  const [recipient, setRecipient] = useState('');
  const [amount, setAmount] = useState('');
  const [status, setStatus] = useState<'idle' | 'proving' | 'submitting' | 'success' | 'error'>('idle');

  async function handleTransfer(e: React.FormEvent) {
    e.preventDefault();
    if (!api || !keypair) return;

    try {
      setStatus('proving');

      // Fetch current state
      const senderAvail = await api.rpc.state.call(
        'ConfidentialAssetsApi_balance_of',
        api.createType('(u128, AccountId32)', [assetId, sender]).toHex()
      );

      const recipientPk = await api.rpc.state.call(
        'ConfidentialAssetsApi_public_key',
        api.createType('AccountId32', recipient).toHex()
      );

      const recipientPending = await api.query.zkhe.pendingBalanceCommit(assetId, recipient);

      // Generate proof
      const { delta_ct, proof } = await zkheProveSenderTransfer({
        asset_id: assetId,
        sender_sk: keypair.sk,
        sender_pk: keypair.pk,
        recipient_pk: recipientPk.toU8a(),
        amount: BigInt(amount),
        sender_avail_old: senderAvail.toU8a(),
        recipient_pending_old: recipientPending.isSome ? recipientPending.unwrap().toU8a() : new Uint8Array(32),
      });

      setStatus('submitting');

      // Submit transaction
      const injector = await web3FromAddress(sender);
      await api.tx.confidentialAssets.confidentialTransfer(
        assetId,
        recipient,
        delta_ct,
        proof
      ).signAndSend(sender, { signer: injector.signer });

      setStatus('success');
    } catch (err) {
      console.error(err);
      setStatus('error');
    }
  }

  return (
    <form onSubmit={handleTransfer}>
      <input
        type="text"
        placeholder="Recipient address"
        value={recipient}
        onChange={e => setRecipient(e.target.value)}
      />
      <input
        type="number"
        placeholder="Amount"
        value={amount}
        onChange={e => setAmount(e.target.value)}
      />
      <button type="submit" disabled={status === 'proving' || status === 'submitting'}>
        {status === 'proving' ? 'Generating proof...' :
         status === 'submitting' ? 'Submitting...' : 'Transfer'}
      </button>
      {status === 'success' && <p>Transfer successful!</p>}
      {status === 'error' && <p>Transfer failed</p>}
    </form>
  );
}
```

## Event Subscriptions

### Subscribe to Transfer Events

```typescript
async function subscribeToTransfers(
  api: ApiPromise,
  account: string,
  onTransfer: (event: any) => void
) {
  return api.query.system.events((events) => {
    events.forEach((record) => {
      const { event } = record;

      if (api.events.confidentialAssets.ConfidentialTransfer.is(event)) {
        const [asset, from, to, encryptedAmount] = event.data;

        if (from.toString() === account || to.toString() === account) {
          onTransfer({
            type: 'transfer',
            asset: asset.toNumber(),
            from: from.toString(),
            to: to.toString(),
            encryptedAmount: encryptedAmount.toU8a(),
          });
        }
      }

      if (api.events.confidentialAssets.Deposit.is(event)) {
        const [asset, who, amount] = event.data;
        if (who.toString() === account) {
          onTransfer({
            type: 'deposit',
            asset: asset.toNumber(),
            who: who.toString(),
            amount: amount.toBigInt(),
          });
        }
      }
    });
  });
}
```

## Error Handling

```typescript
enum ConfidentialError {
  NoPk = 'Account has no registered public key',
  InsufficientBalance = 'Insufficient confidential balance',
  InvalidProof = 'ZK proof verification failed',
  PendingNotFound = 'No pending balance to claim',
}

function parseError(error: any): string {
  if (error.isModule) {
    const decoded = api.registry.findMetaError(error.asModule);

    switch (decoded.name) {
      case 'NoPk':
        return ConfidentialError.NoPk;
      case 'ProofVerificationFailed':
        return ConfidentialError.InvalidProof;
      default:
        return decoded.docs.join(' ');
    }
  }
  return error.toString();
}
```

## Caching Strategies

### Balance Cache

```typescript
class BalanceCache {
  private cache = new Map<string, { value: bigint; timestamp: number }>();
  private TTL = 60000; // 1 minute

  key(assetId: number, account: string): string {
    return `${assetId}:${account}`;
  }

  get(assetId: number, account: string): bigint | null {
    const entry = this.cache.get(this.key(assetId, account));
    if (!entry) return null;
    if (Date.now() - entry.timestamp > this.TTL) {
      this.cache.delete(this.key(assetId, account));
      return null;
    }
    return entry.value;
  }

  set(assetId: number, account: string, value: bigint) {
    this.cache.set(this.key(assetId, account), { value, timestamp: Date.now() });
  }

  invalidate(assetId: number, account: string) {
    this.cache.delete(this.key(assetId, account));
  }
}
```

## Performance Tips

1. **Batch Queries**: Use `api.queryMulti()` for multiple balance queries
2. **Worker Threads**: Run proof generation in Web Workers
3. **Connection Pooling**: Reuse API connections
4. **Proof Caching**: Cache proofs for retry scenarios (with nonce management)

```typescript
// Web Worker for proof generation
// prover.worker.ts
import { zkheProveSenderTransfer } from 'zkhe-prover-wasm';

self.onmessage = async (e) => {
  const { type, input } = e.data;

  try {
    let result;
    switch (type) {
      case 'transfer':
        result = await zkheProveSenderTransfer(input);
        break;
      // ... other proof types
    }
    self.postMessage({ success: true, result });
  } catch (error) {
    self.postMessage({ success: false, error: error.message });
  }
};

// Main thread usage
const proverWorker = new Worker('./prover.worker.ts');

function generateProofInWorker(type: string, input: any): Promise<any> {
  return new Promise((resolve, reject) => {
    proverWorker.onmessage = (e) => {
      if (e.data.success) resolve(e.data.result);
      else reject(new Error(e.data.error));
    };
    proverWorker.postMessage({ type, input });
  });
}
```

## Next Steps

- [Testing Guide](./testing.md) - Test your client integration
- [API Reference](./api.md) - Complete API documentation
- [Architecture](./architecture.md) - Understand the system design
