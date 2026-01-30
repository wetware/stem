//! ABI types and decoding for the Stem contract.
//!
//! HeadUpdated event and head() view. Decode from JSON-RPC log shape and eth_call return.

use anyhow::{Context, Result};
use serde_json::Value;

/// CIDKind matches Solidity `CIDKind` declaration order (discriminant = uint8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum CidKind {
    IPFS_UNIXFS = 0,
    IPLD_NODE = 1,
    BLOB = 2,
    IPNS_NAME = 3,
}

impl CidKind {
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            0 => Some(CidKind::IPFS_UNIXFS),
            1 => Some(CidKind::IPLD_NODE),
            2 => Some(CidKind::BLOB),
            3 => Some(CidKind::IPNS_NAME),
            _ => None,
        }
    }
}

/// First 4 bytes of keccak256("HeadUpdated(uint64,address,uint8,bytes,bytes32)").
pub const HEAD_UPDATED_TOPIC0: [u8; 4] = [0xab, 0xb9, 0xa0, 0x0f];

/// Selector for head().
pub const HEAD_SELECTOR: [u8; 4] = [0x8f, 0x7d, 0xcf, 0xa3];

/// Observed HeadUpdated event with chain metadata (observed-only; no reorg safety).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadUpdatedObserved {
    pub seq: u64,
    pub writer: [u8; 20],
    pub hint: CidKind,
    pub cid: Vec<u8>,
    pub cid_hash: [u8; 32],
    pub block_number: u64,
    pub tx_hash: [u8; 32],
    pub log_index: u64,
}

/// Current head state (from head() or from events).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentHead {
    pub seq: u64,
    pub hint: CidKind,
    pub cid: Vec<u8>,
}

/// Decode a JSON-RPC log (eth_subscription / eth_getLogs result) into HeadUpdatedObserved.
pub fn decode_log_to_observed(log_value: &Value) -> Result<HeadUpdatedObserved> {
    let block_number = parse_hex_u64(
        log_value
            .get("blockNumber")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing blockNumber"))?,
    )?;
    let log_index = parse_hex_u64(
        log_value
            .get("logIndex")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing logIndex"))?,
    )?;
    let tx_hash = parse_hex_bytes_32(
        log_value
            .get("transactionHash")
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing transactionHash"))?,
    )?;
    let data = parse_hex_bytes(
        log_value
            .get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing data"))?,
    )?;
    let topics = log_value
        .get("topics")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow::anyhow!("Missing topics"))?;
    if topics.len() < 4 {
        anyhow::bail!("Expected at least 4 topics, got {}", topics.len());
    }
    let seq = {
        let t1 = parse_hex_bytes(topics[1].as_str().ok_or_else(|| anyhow::anyhow!("topic1 not str"))?)?;
        if t1.len() < 8 {
            anyhow::bail!("topic1 too short for uint64");
        }
        u64::from_be_bytes(t1[t1.len() - 8..].try_into().unwrap())
    };
    // writer is indexed (topics[2]); log "address" is the contract that emitted the event.
    let writer = parse_hex_bytes_20(
        topics[2]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("topic2 not str"))?,
    )?;
    let cid_hash = parse_hex_bytes_32(
        topics[3]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("topic3 not str"))?,
    )?;
    if data.len() < 64 {
        anyhow::bail!("Data too short for (uint8, bytes)");
    }
    let hint_byte = data[31];
    let hint = CidKind::from_u8(hint_byte)
        .ok_or_else(|| anyhow::anyhow!("Invalid CidKind {}", hint_byte))?;
    let offset = u32::from_be_bytes(data[32..36].try_into().unwrap()) as usize;
    if data.len() < offset + 32 {
        anyhow::bail!("Data too short for bytes at offset {}", offset);
    }
    let len_word_start = offset;
    let len = u32::from_be_bytes(data[len_word_start + 28..len_word_start + 32].try_into().unwrap()) as usize;
    if data.len() < offset + 32 + len {
        anyhow::bail!("Data too short for cid len {}", len);
    }
    let cid = data[offset + 32..offset + 32 + len].to_vec();

    Ok(HeadUpdatedObserved {
        seq,
        writer,
        hint,
        cid,
        cid_hash,
        block_number,
        tx_hash,
        log_index,
    })
}

/// Decode head() return data (eth_call result): (uint64, uint8, bytes) ABI.
pub fn decode_head_return(data: &[u8]) -> Result<CurrentHead> {
    if data.len() < 96 {
        anyhow::bail!("head() return too short");
    }
    let seq = u64::from_be_bytes(data[24..32].try_into().unwrap());
    let hint_byte = data[63];
    let hint = CidKind::from_u8(hint_byte)
        .ok_or_else(|| anyhow::anyhow!("Invalid CidKind {}", hint_byte))?;
    let cid_offset = u32::from_be_bytes(data[64..68].try_into().unwrap()) as usize;
    if data.len() < cid_offset + 32 {
        anyhow::bail!("head() return too short for cid offset");
    }
    let cid_len = u32::from_be_bytes(data[cid_offset + 28..cid_offset + 32].try_into().unwrap()) as usize;
    if data.len() < cid_offset + 32 + cid_len {
        anyhow::bail!("head() return too short for cid");
    }
    let cid = data[cid_offset + 32..cid_offset + 32 + cid_len].to_vec();
    Ok(CurrentHead { seq, hint, cid })
}

fn parse_hex_u64(s: &str) -> Result<u64> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16).context("parse hex u64")
}

fn parse_hex_bytes(s: &str) -> Result<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).context("parse hex bytes")
}

fn parse_hex_bytes_32(s: &str) -> Result<[u8; 32]> {
    let bytes = parse_hex_bytes(s)?;
    if bytes.len() != 32 {
        anyhow::bail!("Expected 32 bytes, got {}", bytes.len());
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn parse_hex_bytes_20(s: &str) -> Result<[u8; 20]> {
    let bytes = parse_hex_bytes(s)?;
    if bytes.len() == 20 {
        let mut out = [0u8; 20];
        out.copy_from_slice(&bytes);
        Ok(out)
    } else if bytes.len() == 32 {
        // Indexed address in EVM is 32 bytes (right-padded); take last 20.
        let mut out = [0u8; 20];
        out.copy_from_slice(&bytes[12..32]);
        Ok(out)
    } else {
        anyhow::bail!("Expected 20 or 32 bytes for address, got {}", bytes.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cid_kind_discriminants() {
        assert_eq!(CidKind::IPFS_UNIXFS as u8, 0);
        assert_eq!(CidKind::IPLD_NODE as u8, 1);
        assert_eq!(CidKind::BLOB as u8, 2);
        assert_eq!(CidKind::IPNS_NAME as u8, 3);
    }

    #[test]
    fn topic0_constant() {
        assert_eq!(HEAD_UPDATED_TOPIC0, [0xab, 0xb9, 0xa0, 0x0f]);
    }
}
