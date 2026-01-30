//! Off-chain Stem runtime: head-following and indexing for the Stem contract.
//!
//! This crate provides StemIndexer: observed-only indexing of HeadUpdated events
//! (no reorg safety or confirmations in this iteration).

pub mod abi;
pub mod config;
pub mod cursor;
pub mod indexer;

pub use abi::{CidKind, CurrentHead, HeadUpdatedObserved};
pub use config::{IndexerConfig, ReconnectionConfig};
pub use cursor::Cursor;
pub use indexer::StemIndexer;

/// Current head state (alias for ABI CurrentHead).
pub type Head = CurrentHead;

#[cfg(test)]
mod tests {
    #[test]
    fn stub() {}
}
