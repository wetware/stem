//! IPLD Trie Root validation (offline).
//!
//! Validates that a provided IPLD root node conforms to the TrieRoot v0 schema.
//! No CID resolution or network access; input is already-fetched IPLD bytes.
//!
//! **Encoding assumption (v0):** We assume the root node is encoded as **DAG-CBOR**
//! (IPLD codec 0x71). DAG-CBOR is a CBOR subset with canonical map key ordering;
//! decoding accepts any CBOR map with the required keys and types.

use ciborium::value::Value;
use std::collections::BTreeMap;
use std::io::Cursor;
use thiserror::Error;

/// TrieRoot v0: minimal header for a bit-partitioned vector trie.
///
/// Required keys: `schema` (0), `fanout` (>0), `root` (opaque bytes or CID string), `size` (>=0).
/// Optional: `meta` (map; ignored in v0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrieRootV0 {
    /// Fanout of the trie (must be > 0).
    pub fanout: u64,
    /// Opaque root reference: CID bytes or CID string as bytes. Not parsed.
    pub root: Vec<u8>,
    /// Total size (element count or byte length; must be >= 0).
    pub size: u64,
}

/// Errors produced when validating a TrieRoot v0 payload.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrieError {
    #[error("CBOR decode error: {0}")]
    Decode(String),

    #[error("root value is not a map")]
    NotAMap,

    #[error("missing required key: {0}")]
    MissingKey(&'static str),

    #[error("wrong type for key '{0}': expected {1}")]
    WrongType(&'static str, &'static str),

    #[error("schema version must be 0, got {0}")]
    WrongSchema(i128),

    #[error("fanout must be > 0, got {0}")]
    InvalidFanout(i128),

    #[error("size must be >= 0, got {0}")]
    InvalidSize(i128),
}

/// Validates that `bytes` is DAG-CBOR-encoded TrieRoot v0.
///
/// Input is the raw bytes of an IPLD root node (already fetched). No CID resolution.
pub fn validate_trie_root_v0(bytes: &[u8]) -> Result<TrieRootV0, TrieError> {
    let value: Value = ciborium::de::from_reader(Cursor::new(bytes))
        .map_err(|e| TrieError::Decode(e.to_string()))?;

    let map = match &value {
        Value::Map(m) => m,
        _ => return Err(TrieError::NotAMap),
    };

    // Build a string-keyed lookup (DAG-CBOR maps use text keys for our schema).
    let lookup: BTreeMap<String, &Value> = map
        .iter()
        .filter_map(|(k, v)| {
            if let Value::Text(s) = k {
                Some((s.clone(), v))
            } else {
                None
            }
        })
        .collect();

    let get = |key: &'static str| lookup.get(key).copied();

    // Required: "schema" -> integer, must equal 0
    let schema_val = get("schema").ok_or(TrieError::MissingKey("schema"))?;
    let schema = as_i128(schema_val).ok_or(TrieError::WrongType("schema", "integer"))?;
    if schema != 0 {
        return Err(TrieError::WrongSchema(schema));
    }

    // Required: "fanout" -> integer, must be > 0
    let fanout_val = get("fanout").ok_or(TrieError::MissingKey("fanout"))?;
    let fanout = as_i128(fanout_val).ok_or(TrieError::WrongType("fanout", "integer"))?;
    if fanout <= 0 {
        return Err(TrieError::InvalidFanout(fanout));
    }

    // Required: "root" -> bytes or text (opaque); store as Vec<u8>
    let root_val = get("root").ok_or(TrieError::MissingKey("root"))?;
    let root = match root_val {
        Value::Bytes(b) => b.clone(),
        Value::Text(s) => s.as_bytes().to_vec(),
        _ => return Err(TrieError::WrongType("root", "bytes or string")),
    };

    // Required: "size" -> integer, must be >= 0
    let size_val = get("size").ok_or(TrieError::MissingKey("size"))?;
    let size = as_i128(size_val).ok_or(TrieError::WrongType("size", "integer"))?;
    if size < 0 {
        return Err(TrieError::InvalidSize(size));
    }

    // Optional: "meta" -> map (ignored in v0); no need to validate

    Ok(TrieRootV0 {
        fanout: fanout as u64,
        root,
        size: size as u64,
    })
}

fn as_i128(v: &Value) -> Option<i128> {
    match v {
        Value::Integer(i) => (*i).try_into().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value;

    /// Encode a map as CBOR (key order for DAG-CBOR: lexicographic by key bytes).
    fn encode_map(entries: &[(&str, Value)]) -> Vec<u8> {
        let mut map: Vec<(Value, Value)> = entries
            .iter()
            .map(|(k, v)| (Value::Text((*k).to_string()), v.clone()))
            .collect();
        map.sort_by(|a, b| {
            let a = a.0.as_text().unwrap_or_default();
            let b = b.0.as_text().unwrap_or_default();
            a.as_bytes().cmp(b.as_bytes())
        });
        let value = Value::Map(map);
        let mut out = Vec::new();
        ciborium::ser::into_writer(&value, &mut out).unwrap();
        out
    }

    #[test]
    fn valid_trie_root_v0_succeeds() {
        let cbor = encode_map(&[
            ("fanout", Value::Integer(8.into())),
            ("root", Value::Bytes(vec![0x01, 0x71, 0x00, 0x01, 0x02, 0x03])),
            ("schema", Value::Integer(0.into())),
            ("size", Value::Integer(100.into())),
        ]);
        let trie = validate_trie_root_v0(&cbor).unwrap();
        assert_eq!(trie.fanout, 8);
        assert_eq!(trie.root, vec![0x01, 0x71, 0x00, 0x01, 0x02, 0x03]);
        assert_eq!(trie.size, 100);
    }

    #[test]
    fn valid_trie_root_v0_root_as_string_succeeds() {
        let cbor = encode_map(&[
            ("fanout", Value::Integer(16.into())),
            (
                "root",
                Value::Text("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_string()),
            ),
            ("schema", Value::Integer(0.into())),
            ("size", Value::Integer(0.into())),
        ]);
        let trie = validate_trie_root_v0(&cbor).unwrap();
        assert_eq!(trie.fanout, 16);
        assert_eq!(
            trie.root,
            "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".as_bytes()
        );
        assert_eq!(trie.size, 0);
    }

    #[test]
    fn valid_trie_root_v0_with_meta_ignored() {
        let meta = Value::Map(vec![(
            Value::Text("foo".to_string()),
            Value::Text("bar".to_string()),
        )]);
        let cbor = encode_map(&[
            ("fanout", Value::Integer(2.into())),
            ("meta", meta),
            ("root", Value::Bytes(vec![])),
            ("schema", Value::Integer(0.into())),
            ("size", Value::Integer(42.into())),
        ]);
        let trie = validate_trie_root_v0(&cbor).unwrap();
        assert_eq!(trie.fanout, 2);
        assert!(trie.root.is_empty());
        assert_eq!(trie.size, 42);
    }

    #[test]
    fn missing_key_fails() {
        let cbor = encode_map(&[
            ("fanout", Value::Integer(8.into())),
            ("root", Value::Bytes(vec![1, 2, 3])),
            // missing "schema"
            ("size", Value::Integer(0.into())),
        ]);
        let err = validate_trie_root_v0(&cbor).unwrap_err();
        assert_eq!(err, TrieError::MissingKey("schema"));
    }

    #[test]
    fn wrong_schema_version_fails() {
        let cbor = encode_map(&[
            ("fanout", Value::Integer(8.into())),
            ("root", Value::Bytes(vec![])),
            ("schema", Value::Integer(1.into())), // must be 0
            ("size", Value::Integer(0.into())),
        ]);
        let err = validate_trie_root_v0(&cbor).unwrap_err();
        assert_eq!(err, TrieError::WrongSchema(1));
    }

    #[test]
    fn wrong_type_schema_string_fails() {
        let cbor = encode_map(&[
            ("fanout", Value::Integer(8.into())),
            ("root", Value::Bytes(vec![])),
            ("schema", Value::Text("0".to_string())), // should be integer
            ("size", Value::Integer(0.into())),
        ]);
        let err = validate_trie_root_v0(&cbor).unwrap_err();
        assert_eq!(err, TrieError::WrongType("schema", "integer"));
    }

    #[test]
    fn fanout_zero_fails() {
        let cbor = encode_map(&[
            ("fanout", Value::Integer(0.into())),
            ("root", Value::Bytes(vec![])),
            ("schema", Value::Integer(0.into())),
            ("size", Value::Integer(0.into())),
        ]);
        let err = validate_trie_root_v0(&cbor).unwrap_err();
        assert_eq!(err, TrieError::InvalidFanout(0));
    }

    #[test]
    fn fanout_negative_fails() {
        let cbor = encode_map(&[
            ("fanout", Value::Integer((-1i64).into())),
            ("root", Value::Bytes(vec![])),
            ("schema", Value::Integer(0.into())),
            ("size", Value::Integer(0.into())),
        ]);
        let err = validate_trie_root_v0(&cbor).unwrap_err();
        assert_eq!(err, TrieError::InvalidFanout(-1));
    }

    #[test]
    fn size_negative_fails() {
        let cbor = encode_map(&[
            ("fanout", Value::Integer(8.into())),
            ("root", Value::Bytes(vec![])),
            ("schema", Value::Integer(0.into())),
            ("size", Value::Integer((-1i64).into())),
        ]);
        let err = validate_trie_root_v0(&cbor).unwrap_err();
        assert_eq!(err, TrieError::InvalidSize(-1));
    }

    #[test]
    fn not_a_map_fails() {
        // Use an array instead of a map to trigger NotAMap.
        let value = Value::Array(vec![Value::Integer(0.into())]);
        let mut out = Vec::new();
        ciborium::ser::into_writer(&value, &mut out).unwrap();
        let err = validate_trie_root_v0(&out).unwrap_err();
        assert_eq!(err, TrieError::NotAMap);
    }
}
