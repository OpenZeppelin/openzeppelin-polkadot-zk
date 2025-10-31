# Principles To Keep Things Sane

## Impl Confidentiality NOT Anonymity
- do NOT enable user anonymity; account addresses are public and only amounts are private
- do implement confidentiality by encrypting balances with owner keys

## Impl On-Chain Verification For Encrypted Balance Updates
- updates to encrypted balances enforce expected rules (i.e. conservation of supply for transfers, etc) by executing on-chain verification of Zero Knowledge proofs (generated using the owner keys)

## Keep Separate Paths for Confidential and Public Assets
- keep confidential assets and operations using them separate from public multi-asset balances and operations i.e. `pallet-{confidential-,}{htlc, escrow}`