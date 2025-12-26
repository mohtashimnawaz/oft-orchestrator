// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/MyOFT.sol";

contract DeployOFT is Script {
    function run(address _lzEndpoint) external returns (address) {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        MyOFT oft = new MyOFT(
            "Synthetix Solana Token", 
            "SOL-SYN", 
            _lzEndpoint, 
            vm.addr(deployerPrivateKey)
        );

        vm.stopBroadcast();
        
        // CRITICAL: This specific log format is parsed by Rust
        console.log("DEPLOYED_ADDR:", address(oft));
        
        return address(oft);
    }
}