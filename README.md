# Stem

Off-chain runtime for the [Stem smart contract](src/Stem.sol).

Stem indexes `HeadUpdated` events emitted by an on-chain anchor contract,
finalizes them with reorg safety (configurable confirmation depth), and exposes
epoch-scoped authority to clients via Cap'n Proto capabilities. When the
on-chain head advances, every capability issued under the previous epoch
hard-fails — clients re-graft to recover.

## Architecture

```
HeadUpdated → Indexer → Finalizer → Epoch → Membrane
                                               │
                                          graft(signer)
                                               │
                                            Session
                                         ┌─────┴─────┐
                                   statusPoller    (future caps)
                                         │
                                    Ok / RPC error
```

The runtime is a four-stage pipeline:

### 1. Stem contract (`src/Stem.sol`)

On-chain anchor. The owner calls `setHead(newCid)` to advance a monotonic
`seq` and emit a `HeadUpdated` event. The `head()` view returns the canonical
`(seq, cid)` pair for cross-checking.

```solidity
event HeadUpdated(
    uint64 indexed seq,
    address indexed writer,
    bytes cid,
    bytes32 indexed cidHash
);
```

### 2. Indexer (`StemIndexer`)

Subscribes to `HeadUpdated` via WebSocket for live events and backfills
missed blocks via HTTP `eth_getLogs` on startup and reconnect. Broadcasts
`HeadUpdatedObserved` values to downstream consumers. Reconnects with
exponential backoff and jitter. Client-side filtering handles RPC nodes
(e.g. Anvil) that don't support topic filters natively.

The indexer is observation-only — it makes no reorg-safety guarantees.

### 3. Finalizer (`Finalizer` / `FinalizerBuilder`)

Consumes observed events from the indexer and outputs only those that are
**eligible** and **canonical**:

- **Eligibility** is decided by a pluggable `Strategy` trait. The built-in
  `ConfirmationDepth(K)` strategy requires `tip >= event.block_number + K`.
- **Canonical cross-check**: after eligibility, the finalizer calls
  `Stem.head()` and only emits if the on-chain `(seq, cid)` matches the
  candidate event.
- **Deduplication** by `(tx_hash, log_index)` ensures exactly-once delivery
  across reconnects and backfills.

Each output is a `FinalizedEvent` containing `seq`, `cid`, `block_number`,
`tx_hash`, `log_index`, and `writer`.

### 4. Membrane (`MembraneServer` / Cap'n Proto RPC)

The capability layer. A `MembraneServer` holds a `watch::Receiver<Epoch>`
and exposes a single entry point:

```
graft(signer) → Session { issuedEpoch, statusPoller }
```

All capabilities inside a `Session` share an `EpochGuard` that checks
`current.seq == issued_seq` on every RPC call. When the epoch advances,
every outstanding capability fails with a `staleEpoch` RPC error. Clients
call `graft()` again to obtain a fresh session under the new epoch.

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Foundry](https://getfoundry.sh/) (forge, anvil, cast)
- [Cap'n Proto compiler](https://capnproto.org/install.html) (`capnp`)

### Build

```bash
forge build
cargo build -p stem
```

### Test

```bash
forge test
cargo test -p stem
```

## Deploy (local)

Start Anvil in one terminal:

```bash
anvil
```

Deploy the contract and set the first head:

```bash
# Deploy
forge script script/Deploy.s.sol \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

# Note the deployed address from the output, then set the first head:
cast send <STEM_ADDRESS> "setHead(bytes)" "0x$(echo -n 'ipfs://first' | xxd -p)" \
  --rpc-url http://127.0.0.1:8545 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
```

## Examples

All examples live in `crates/stem/examples/` and connect to a running node
with a deployed Stem contract.

### `stem_indexer` — raw observed events

Prints every `HeadUpdated` event as it arrives (no finalization).

```bash
cargo run -p stem --example stem_indexer -- \
  --rpc-url http://127.0.0.1:8545 \
  --contract <STEM_ADDRESS>
```

### `finalizer` — finalized events as JSON

Runs the full indexer + finalizer pipeline and prints one JSON object per
finalized event.

```bash
cargo run -p stem --example finalizer -- \
  --ws-url ws://127.0.0.1:8545 \
  --http-url http://127.0.0.1:8545 \
  --contract <STEM_ADDRESS> \
  --depth 2
```

### `membrane_poll` — epoch expiration and re-graft

Demonstrates the full pipeline: indexer, finalizer, membrane, graft, poll.
When a second `setHead` is finalized the existing session's `statusPoller`
fails with a `staleEpoch` error; the example re-grafts and polls successfully
under the new epoch.

```bash
cargo run -p stem --example membrane_poll -- \
  --ws-url ws://127.0.0.1:8545 \
  --http-url http://127.0.0.1:8545 \
  --contract <STEM_ADDRESS> \
  --depth 2
```

## Cap'n Proto schema

The RPC interface is defined in [`capnp/stem.capnp`](capnp/stem.capnp):

| Type | Kind | Description |
|------|------|-------------|
| `Epoch` | struct | `seq`, `head`, `adoptedBlock` — identifies a finalized head |
| `Status` | enum | `ok`, `unauthorized`, `internalError` |
| `Signer` | interface | `sign(domain, nonce) → sig` — client-supplied signing capability |
| `StatusPoller` | interface | `pollStatus() → status` — epoch-scoped health check |
| `Session` | struct | `issuedEpoch`, `statusPoller` — returned by `graft` |
| `Membrane` | interface | `graft(signer) → session` — the sole entry point |

## License

MIT
