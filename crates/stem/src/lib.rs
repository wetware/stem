//! Off-chain Stem runtime: head-following and indexing for the Stem contract.
//!
//! This crate provides StemIndexer: observed-only indexing of HeadUpdated events
//! (no reorg safety or confirmations in this iteration).

pub mod abi;
pub mod config;
pub mod cursor;
pub mod finalizer;
pub mod indexer;
pub mod trie;

pub use abi::{CurrentHead, HeadUpdatedObserved};
pub use config::{IndexerConfig, ReconnectionConfig};
pub use cursor::Cursor;
pub use finalizer::{
    ConfirmationDepth, FinalizedEvent, Finalizer, FinalizerBuilder, FinalizerError, Strategy,
};
pub use indexer::StemIndexer;
pub use trie::{validate_trie_root_v0, TrieError, TrieRootV0};

/// Current head state (alias for ABI CurrentHead).
pub type Head = CurrentHead;

#[cfg(test)]
mod tests {
    #[test]
    fn stub() {}
}
