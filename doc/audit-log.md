# Audit log structure and validation boundaries

Canonical reference for the Stem head → trie pipeline and where each validation boundary sits. This doc defines when data stops being “just bytes” and becomes a parsed CID, and when it becomes a known-valid trie.

---

## Pipeline overview

```
Chain (Stem.head / HeadUpdated)
    → head bytes (opaque)
    → [caller: parse as CID, resolve]
    → IPLD block bytes (opaque)
    → validate_trie_root_v0
    → TrieRootV0 (known-valid trie root)
```

Stem’s **only** validation boundary is the last step. Everything before that is opaque bytes as far as this crate is concerned.

---

## 1. Head bytes (from chain)

**Source:** `Stem.head()` or `HeadUpdated.cid` (and `cidHash`).

**What it is:** Opaque bytes. On-chain they represent “the current head” (typically a CID, but the contract does not interpret them).

**Validation boundary (bytes → parsed CID):** **Not in this crate.** Stem never parses head bytes as a CID (no multibase/multicodec decoding). The boundary “bytes become a parsed CID” is the responsibility of the caller or another layer (e.g. a resolver that interprets these bytes as a CID and fetches the block). Stem only stores and passes through `Vec<u8>`.

**Stem’s contract:** Indexer and ABI expose `seq` and `cid` (bytes). No guarantee that `cid` is a valid CID; no resolution. The crate also provides a **Finalizer** that consumes indexer output and emits only events that pass a configurable Strategy (e.g. confirmation depth) and the canonical head cross-check (`Stem.head()`), giving reorg-safe finalized output.

---

## 2. IPLD block bytes (after resolution)

**Source:** Caller resolves the head (e.g. as a CID) and fetches the corresponding IPLD block. Result is raw bytes (e.g. DAG-CBOR).

**What it is:** Opaque bytes of one IPLD block. For our trie, this block is expected to be a TrieRoot v0 node (DAG-CBOR).

**Validation boundary (bytes → known-valid trie):** **`validate_trie_root_v0(bytes)` in `crates/stem/src/trie.rs`.**

- **Input:** Raw bytes of a single IPLD node (already fetched). No CID resolution or I/O inside this function.
- **Output:** `TrieRootV0 { fanout, root, size }` or `TrieError`.
- **Meaning:** Before the call, the bytes are unvalidated (could be anything). After a successful call, you have a **known-valid TrieRoot v0** shape: DAG-CBOR map with required keys and types. The function does not fetch or follow links; it does not parse `TrieRootV0.root` as a CID.

So: **“Where does it stop being a CID and start being a known-valid trie?”** — It stops being “just bytes” and becomes a known-valid trie **at the return of `validate_trie_root_v0`**. The *input* to that function is the bytes you got from somewhere (e.g. after resolving a CID); the *output* is the validated trie root structure.

---

## 3. TrieRoot v0 (audit log structure)

**Schema (TrieRoot v0):** A single IPLD node encoded as **DAG-CBOR** (codec 0x71). One map with:

| Key     | Type    | Required | Meaning |
|---------|---------|----------|---------|
| `schema`| integer | yes      | Must be `0` |
| `fanout`| integer | yes      | Trie fanout; must be > 0 |
| `root`  | bytes or string | yes | Opaque root reference (CID bytes or CID string); not parsed by stem |
| `size`  | integer | yes      | Total size (e.g. element count); must be ≥ 0 |
| `meta`  | map     | no       | Ignored in v0 |

**After validation:** You have a `TrieRootV0` with `fanout`, `root` (still opaque bytes), and `size`. Traversing the trie (following `root`, interpreting children) is outside stem’s validation; stem only validates this root node’s shape.

---

## Summary

| Stage            | Data form           | Where boundary is                         |
|------------------|---------------------|-------------------------------------------|
| Chain → head     | bytes (opaque)      | N/A in stem                               |
| Bytes → parsed CID | Parsed CID        | **Caller / other layer** (not in stem)    |
| CID → block bytes | Fetched bytes      | **Caller / resolver** (not in stem)       |
| Bytes → known-valid trie | TrieRootV0      | **`validate_trie_root_v0`** in stem       |

Stem’s only validation boundary is: **raw IPLD bytes in → TrieRoot v0 structure out** (or error). No CID parsing, no resolution, no traversal.
