# Stem contract ABI reference

Canonical reference for off-chain consumers (indexer, tools) implementing against the Stem contract. Source: [`src/Stem.sol`](../src/Stem.sol).

## CIDKind enum (Solidity declaration order)

```solidity
enum CIDKind { IPFS_UNIXFS, IPLD_NODE, BLOB, IPNS_NAME }
```

- IPFS_UNIXFS = 0, IPLD_NODE = 1, BLOB = 2, IPNS_NAME = 3

## HeadUpdated event

```solidity
event HeadUpdated(
    uint64 indexed seq,
    address indexed writer,
    CIDKind hint,
    bytes cid,
    bytes32 indexed cidHash
);
```

- Indexed: seq (topic1), writer (topic2), cidHash (topic3)
- Non-indexed in data: (uint8 hint, bytes cid) ABI-encoded
- Topic0 (first 4 bytes of keccak256): `0xabb9a00f` → `[0xab, 0xb9, 0xa0, 0x0f]`

## head() view

```solidity
function head() external view returns (uint64 currentSeq, CIDKind currentHint, bytes memory cid);
```

- Selector: `0x8f7dcfa3`
- Return ABI: (uint64, uint8, bytes)
