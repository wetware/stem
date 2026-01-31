//! ABI types and decoding for the Stem contract.
//!
//! HeadUpdated event and head() view. Decode from JSON-RPC log shape and eth_call return.
//! Option A: no CIDKind/hint; head() returns (uint64, bytes), event HeadUpdated(seq, writer, cid, cidHash).

use anyhow::{Context, Result};
use serde_json::Value;

/// First 4 bytes of keccak256("HeadUpdated(uint64,address,bytes,bytes32)").
pub const HEAD_UPDATED_TOPIC0: [u8; 4] = [0x85, 0xf2, 0xcb, 0x2e];

/// Selector for head().
pub const HEAD_SELECTOR: [u8; 4] = [0x8f, 0x7d, 0xcf, 0xa3];

/// Observed HeadUpdated event with chain metadata (observed-only; no reorg safety).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadUpdatedObserved {
    pub seq: u64,
    pub writer: [u8; 20],
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
    pub cid: Vec<u8>,
}

/// Decode a JSON-RPC log (eth_subscription / eth_getLogs result) into HeadUpdatedObserved.
/// Option A: event HeadUpdated(uint64 indexed seq, address indexed writer, bytes cid, bytes32 indexed cidHash).
/// Data is ABI-encoded single bytes: offset (32) then at offset: length then cid.
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
    // Option A: data is single bytes: offset (32 bytes) then at offset: length (32) then cid
    if data.len() < 64 {
        anyhow::bail!("Data too short for bytes");
    }
    let len = u32::from_be_bytes(data[60..64].try_into().unwrap()) as usize;
    if data.len() < 64 + len {
        anyhow::bail!("Data too short for cid len {}", len);
    }
    let cid = data[64..64 + len].to_vec();

    Ok(HeadUpdatedObserved {
        seq,
        writer,
        cid,
        cid_hash,
        block_number,
        tx_hash,
        log_index,
    })
}

/// Decode head() return data (eth_call result): (uint64, bytes) ABI.
pub fn decode_head_return(data: &[u8]) -> Result<CurrentHead> {
    if data.len() < 64 {
        anyhow::bail!("head() return too short");
    }
    let seq = u64::from_be_bytes(data[24..32].try_into().unwrap());
    let cid_offset = u32::from_be_bytes(data[32..36].try_into().unwrap()) as usize;
    if data.len() < cid_offset + 32 {
        anyhow::bail!("head() return too short for cid offset");
    }
    let cid_len = u32::from_be_bytes(data[cid_offset + 28..cid_offset + 32].try_into().unwrap()) as usize;
    if data.len() < cid_offset + 32 + cid_len {
        anyhow::bail!("head() return too short for cid");
    }
    let cid = data[cid_offset + 32..cid_offset + 32 + cid_len].to_vec();
    Ok(CurrentHead { seq, cid })
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
    fn topic0_constant() {
        assert_eq!(HEAD_UPDATED_TOPIC0, [0x85, 0xf2, 0xcb, 0x2e]);
    }

    /// Minimal ABI-encoded head() return: (uint64 seq, bytes cid) with seq=0, cid=[].
    #[test]
    fn decode_head_return_minimal() {
        let mut data = [0u8; 96];
        data[24..32].copy_from_slice(&0u64.to_be_bytes()); // seq
        data[32..36].copy_from_slice(&64u32.to_be_bytes()); // offset to cid
        let head = decode_head_return(&data).unwrap();
        assert_eq!(head.seq, 0);
        assert!(head.cid.is_empty());
    }

    /// Option A: (uint64, bytes); cid at offset 64, length at 92..96, cid bytes at 96..
    #[test]
    fn decode_head_return_with_cid() {
        let seq: u64 = 42;
        let cid: &[u8] = b"QmFoo";
        let mut data = vec![0u8; 96 + cid.len()];
        data[24..32].copy_from_slice(&seq.to_be_bytes());
        data[32..36].copy_from_slice(&64u32.to_be_bytes()); // offset to cid
        data[92..96].copy_from_slice(&(cid.len() as u32).to_be_bytes()); // length at 64+28..64+32
        data[96..96 + cid.len()].copy_from_slice(cid);
        let head = decode_head_return(&data).unwrap();
        assert_eq!(head.seq, 42);
        assert_eq!(head.cid, cid);
    }
}
