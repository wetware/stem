//! This crate will implement head-following, finalization, and caching for the Stem contract.

/// Classification of how runtimes should interpret the head pointer.
/// Mirrors Solidity `CIDKind`; hint is advisory; off-chain callers must validate defensively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CidKind {
    IpfsUnixfs,
    IpldNode,
    Blob,
    IpnsName,
}

/// Current head state, mirroring the on-chain tuple `(seq, hint, cid)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub seq: u64,
    pub hint: CidKind,
    pub cid: Vec<u8>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn stub() {}
}
