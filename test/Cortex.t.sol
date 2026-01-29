// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import { Test } from "forge-std/Test.sol";
import { Stem } from "../src/Cortex.sol";

contract CortexTest is Test {
    Stem public stem;
    address public owner;
    address public user;

    function setUp() public {
        owner = address(this);
        user = address(0x123);
        stem = new Stem("ipfs://initial");
    }

    function test_Constructor() public view {
        assertEq(stem.owner(), owner);
        assertEq(stem.seq(), 0);
        (uint64 seq, string memory head) = stem.head();
        assertEq(seq, 0);
        assertEq(head, "ipfs://initial");
    }

    function test_SetHead() public {
        vm.expectEmit(true, true, false, true);
        emit Stem.HeadUpdated(1, owner, "ipfs://new");
        
        stem.setHead("ipfs://new");
        
        assertEq(stem.seq(), 1);
        (uint64 seq, string memory head) = stem.head();
        assertEq(seq, 1);
        assertEq(head, "ipfs://new");
    }

    function test_SetHead_NotOwner() public {
        vm.prank(user);
        vm.expectRevert(Stem.NotOwner.selector);
        stem.setHead("ipfs://new");
    }

    function test_SetHead_MultipleUpdates() public {
        stem.setHead("ipfs://first");
        assertEq(stem.seq(), 1);
        
        stem.setHead("ipfs://second");
        assertEq(stem.seq(), 2);
        
        stem.setHead("ipfs://third");
        assertEq(stem.seq(), 3);
        
        (uint64 seq, string memory head) = stem.head();
        assertEq(seq, 3);
        assertEq(head, "ipfs://third");
    }

    function test_TransferOwnership() public {
        stem.transferOwnership(user);
        assertEq(stem.owner(), user);
    }

    function test_TransferOwnership_NotOwner() public {
        vm.prank(user);
        vm.expectRevert(Stem.NotOwner.selector);
        stem.transferOwnership(user);
    }

    function test_TransferOwnership_ThenSetHead() public {
        stem.transferOwnership(user);
        
        vm.prank(user);
        stem.setHead("ipfs://new-owner");
        
        assertEq(stem.seq(), 1);
        (uint64 seq, string memory head) = stem.head();
        assertEq(seq, 1);
        assertEq(head, "ipfs://new-owner");
    }
}
