//! Off-chain Stem runtime: head-following, indexing, and finalization for the Stem contract.
//!
//! - **StemIndexer**: observed-only indexing of HeadUpdated events (WebSocket + HTTP backfill;
//!   no reorg safety or confirmations in the indexer itself).
//! - **Finalizer**: consumes indexer output and emits only events that are eligible per a
//!   configurable [Strategy] (e.g. [ConfirmationDepth]) and pass the canonical cross-check
//!   (`Stem.head()`), giving reorg-safe finalized output.

#[allow(unused_parens)] // generated capnp code
pub mod stem_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/stem_capnp.rs"));
}

pub mod abi;
pub mod config;
pub mod cursor;
pub mod finalizer;
pub mod indexer;
pub mod membrane;
pub mod trie;

pub use abi::{CurrentHead, HeadUpdatedObserved};
pub use config::{IndexerConfig, ReconnectionConfig};
pub use cursor::Cursor;
pub use finalizer::{
    ConfirmationDepth, FinalizedEvent, Finalizer, FinalizerBuilder, FinalizerError, Strategy,
};
pub use indexer::{current_block_number, StemIndexer};
pub use membrane::{membrane_client, Epoch, MembraneServer};
pub use trie::{validate_trie_root_v0, TrieError, TrieRootV0};

/// Current head state (alias for ABI CurrentHead).
pub type Head = CurrentHead;

#[cfg(test)]
mod tests {
    #[test]
    fn stub() {}
}
