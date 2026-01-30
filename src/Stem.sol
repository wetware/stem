// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title Stem
/// @notice The on-chain bootstrap facet of Stem.
///
/// Stem anchors the authoritative "head" pointer for Stem.
/// Off-chain systems (Stem runtime) watch Stem events to learn when
/// the global configuration root has advanced.
///
/// Semantics:
/// - There is exactly one Stem per deployment.
/// - The head is advanced monotonically via `seq`.
/// - Every advance emits an event suitable for deterministic replay.
/// - Authority to advance the head is gated (owner for now).
contract Stem {
    /// @notice Classification of how runtimes should interpret the head pointer.
    /// @dev hint is advisory; off-chain callers must validate the referenced object defensively.
    enum CIDKind { IPFS_UNIXFS, IPLD_NODE, BLOB, IPNS_NAME }

    /// @notice Emitted whenever the Stem head advances.
    /// @param seq     Monotonic sequence number (epoch index)
    /// @param writer  Caller who advanced the head
    /// @param hint    Interpretation hint for the new head
    /// @param cid     New head pointer bytes (binary CID or name bytes)
    /// @param cidHash keccak256(cid), for index-friendly filtering
    event HeadUpdated(
        uint64 indexed seq,
        address indexed writer,
        CIDKind hint,
        bytes cid,
        bytes32 indexed cidHash
    );

    error NotOwner();
    error NoChange();

    /// @notice Current authority allowed to advance the head.
    address public owner;

    /// @notice Monotonic sequence number for head updates.
    uint64 public seq;

    /// @notice Current head interpretation hint (advisory; off-chain callers must validate).
    CIDKind public hint;

    /// @dev Stored head pointer bytes (binary CID or name bytes).
    bytes private _cid;

    /// @param initialHint Interpretation hint for the initial head.
    /// @param initialCid  Initial head pointer bytes.
    /// @dev The initial state is established without emitting an event.
    constructor(CIDKind initialHint, bytes memory initialCid) {
        owner = msg.sender;
        hint = initialHint;
        _cid = initialCid;
        seq = 0;
    }

    /// @notice Returns the current head state.
    /// @return currentSeq  The current sequence number
    /// @return currentHint The current interpretation hint
    /// @return cid         The current head pointer bytes
    function head() external view returns (uint64 currentSeq, CIDKind currentHint, bytes memory cid) {
        return (seq, hint, _cid);
    }

    /// @notice Advance the Stem head.
    /// Emits a HeadUpdated event that off-chain watchers consume.
    function setHead(CIDKind newHint, bytes calldata newCid) external {
        if (msg.sender != owner) revert NotOwner();
        if (newHint == hint && keccak256(newCid) == keccak256(_cid)) revert NoChange();

        unchecked {
            seq += 1;  // u64
        }
        hint = newHint;
        _cid = newCid;

        emit HeadUpdated(seq, msg.sender, newHint, newCid, keccak256(newCid));
    }
}
