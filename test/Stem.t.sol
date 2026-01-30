// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { Stem } from "../src/Stem.sol";

contract StemTest is Test {
    Stem public stem;
    address public owner;
    address public user;

    function setUp() public {
        owner = address(this);
        user = address(0x123);
        stem = new Stem(Stem.CIDKind.IPFS_UNIXFS, bytes("ipfs-initial"));
    }

    function test_Constructor() public view {
        assertEq(stem.owner(), owner);
        assertEq(stem.seq(), 0);
        (uint64 s, Stem.CIDKind h, bytes memory c) = stem.head();
        assertEq(s, 0);
        assertEq(uint8(h), uint8(Stem.CIDKind.IPFS_UNIXFS));
        assertEq(keccak256(c), keccak256(bytes("ipfs-initial")));
    }

    function test_SetHead() public {
        bytes memory newCid = bytes("ipfs://new");
        bytes32 expectedCidHash = keccak256(newCid);
        vm.expectEmit(true, true, true, true);
        emit Stem.HeadUpdated(1, owner, Stem.CIDKind.IPFS_UNIXFS, newCid, expectedCidHash);

        stem.setHead(Stem.CIDKind.IPFS_UNIXFS, newCid);

        assertEq(stem.seq(), 1);
        (uint64 seq, Stem.CIDKind h, bytes memory cid) = stem.head();
        assertEq(seq, 1);
        assertEq(uint8(h), uint8(Stem.CIDKind.IPFS_UNIXFS));
        assertEq(cid, newCid);
    }

    function test_SetHead_NotOwner() public {
        vm.prank(user);
        vm.expectRevert(Stem.NotOwner.selector);
        stem.setHead(Stem.CIDKind.IPFS_UNIXFS, bytes("ipfs://new"));
    }

    function test_SetHead_NoChange() public {
        // Same hint and cid as initial head -> revert NoChange, seq unchanged
        vm.expectRevert(Stem.NoChange.selector);
        stem.setHead(Stem.CIDKind.IPFS_UNIXFS, bytes("ipfs-initial"));
        assertEq(stem.seq(), 0);

        // One real update, then no-op again -> revert NoChange, seq still 1
        stem.setHead(Stem.CIDKind.IPFS_UNIXFS, bytes("ipfs://new"));
        assertEq(stem.seq(), 1);
        vm.expectRevert(Stem.NoChange.selector);
        stem.setHead(Stem.CIDKind.IPFS_UNIXFS, bytes("ipfs://new"));
        assertEq(stem.seq(), 1);
    }

    function test_SetHead_MultipleUpdates() public {
        stem.setHead(Stem.CIDKind.IPFS_UNIXFS, bytes("ipfs://first"));
        assertEq(stem.seq(), 1);

        stem.setHead(Stem.CIDKind.IPLD_NODE, bytes("ipld://second"));
        assertEq(stem.seq(), 2);

        stem.setHead(Stem.CIDKind.BLOB, bytes("blob://third"));
        assertEq(stem.seq(), 3);

        (uint64 seq, Stem.CIDKind h, bytes memory cid) = stem.head();
        assertEq(seq, 3);
        assertEq(uint8(h), uint8(Stem.CIDKind.BLOB));
        assertEq(cid, bytes("blob://third"));
    }

    function test_SetHead_IPFS_UNIXFS() public {
        bytes memory cid = bytes("QmIPFS");
        bytes32 cidHash = keccak256(cid);
        vm.expectEmit(true, true, true, true);
        emit Stem.HeadUpdated(1, owner, Stem.CIDKind.IPFS_UNIXFS, cid, cidHash);

        stem.setHead(Stem.CIDKind.IPFS_UNIXFS, cid);

        assertEq(stem.seq(), 1);
        (uint64 seq, Stem.CIDKind h, bytes memory c) = stem.head();
        assertEq(seq, 1);
        assertEq(uint8(h), uint8(Stem.CIDKind.IPFS_UNIXFS));
        assertEq(c, cid);
    }

    function test_SetHead_IPLD_NODE() public {
        bytes memory cid = bytes("QmIPLD");
        bytes32 cidHash = keccak256(cid);
        vm.expectEmit(true, true, true, true);
        emit Stem.HeadUpdated(1, owner, Stem.CIDKind.IPLD_NODE, cid, cidHash);

        stem.setHead(Stem.CIDKind.IPLD_NODE, cid);

        assertEq(stem.seq(), 1);
        (uint64 seq, Stem.CIDKind h, bytes memory c) = stem.head();
        assertEq(seq, 1);
        assertEq(uint8(h), uint8(Stem.CIDKind.IPLD_NODE));
        assertEq(c, cid);
    }

    function test_SetHead_BLOB() public {
        bytes memory cid = bytes("blob-data");
        bytes32 cidHash = keccak256(cid);
        vm.expectEmit(true, true, true, true);
        emit Stem.HeadUpdated(1, owner, Stem.CIDKind.BLOB, cid, cidHash);

        stem.setHead(Stem.CIDKind.BLOB, cid);

        assertEq(stem.seq(), 1);
        (uint64 seq, Stem.CIDKind h, bytes memory c) = stem.head();
        assertEq(seq, 1);
        assertEq(uint8(h), uint8(Stem.CIDKind.BLOB));
        assertEq(c, cid);
    }

    function test_SetHead_IPNS_NAME() public {
        bytes memory cid = bytes("k51qzi5uqu5...");
        bytes32 cidHash = keccak256(cid);
        vm.expectEmit(true, true, true, true);
        emit Stem.HeadUpdated(1, owner, Stem.CIDKind.IPNS_NAME, cid, cidHash);

        stem.setHead(Stem.CIDKind.IPNS_NAME, cid);

        assertEq(stem.seq(), 1);
        (uint64 seq, Stem.CIDKind h, bytes memory c) = stem.head();
        assertEq(seq, 1);
        assertEq(uint8(h), uint8(Stem.CIDKind.IPNS_NAME));
        assertEq(c, cid);
    }
}
