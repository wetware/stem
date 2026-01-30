# StemIndexer: RPC and WebSocket

Canonical reference for how the Stem indexer talks to the chain (JSON-RPC and WebSocket). Used by `crates/stem` indexer implementation.

## Log filter (eth_getLogs / eth_subscribe)

- **Address:** contract address as `0x{hex}` (20 bytes).
- **Topics:** `[topic0, null, null, null]` where topic0 is the first 4 bytes of the HeadUpdated event signature (see [stem-abi-ref](stem-abi-ref.md)).
- **Block range:** `fromBlock`, `toBlock` as `0x{:x}` when doing backfill.

## eth_subscribe("logs", filter)

- Params: `["logs", filter]`. For providers that reject filter objects (e.g. Anvil), use `["logs"]` and filter client-side by address and topic0.

## Log notification shape (eth_subscription params.result)

- `blockNumber`, `logIndex`, `transactionHash`, `address`, `data`, `topics` — all hex strings. Decode with strip `0x` then hex decode or parse as u64/u32.

## Anvil quirk

- Filter object for `eth_subscribe("logs")` may fail with "data did not match any variant". Fallback: subscribe without filter and filter by address + topic0 in the indexer.
