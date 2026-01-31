// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../src/Stem.sol";

contract Deploy is Script {
    function run() external {
        vm.startBroadcast();
        Stem stem = new Stem(bytes("ipfs-initial"));
        console.log("Stem deployed at:", address(stem));
        vm.stopBroadcast();
    }
}
