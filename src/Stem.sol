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
    /// @notice Emitted whenever the Stem head advances.
    /// @param seq Monotonic sequence number (epoch index)
    /// @param writer Caller who advanced the head
    /// @param head   New head pointer (e.g. ipfs://..., ipns://...)
    event HeadUpdated(
        uint64 indexed seq,
        address indexed writer,
        string head
    );

    error NotOwner();

    /// @notice Current authority allowed to advance the head.
    address public owner;

    /// @notice Monotonic sequence number for head updates.
    uint64 public seq;

    /// @dev Stored head pointer.
    string private _head;

    /// @param initialHead Initial head pointer.
    /// The initial state is established without emitting an event.
    constructor(string memory initialHead) {
        owner = msg.sender;
        _head = initialHead;
        seq = 0;
    }

    /// @notice Returns the current head state.
    /// @return currentSeq The current sequence number
    /// @return headPath   The current head pointer
    function head() external view returns (uint64 currentSeq, string memory headPath) {
        return (seq, _head);
    }

    /// @notice Advance the Stem head.
    /// Emits a HeadUpdated event that off-chain watchers consume.
    function setHead(string calldata newHead) external {
        if (msg.sender != owner) revert NotOwner();

        unchecked {
            seq += 1;  // u64
        }
        _head = newHead;

        emit HeadUpdated(seq, msg.sender, newHead);
    }

    /// @notice Transfer authority to advance the head.
    /// This is a placeholder for future governance / proxy control.
    function transferOwnership(address newOwner) external {
        if (msg.sender != owner) revert NotOwner();
        owner = newOwner;
    }
}
