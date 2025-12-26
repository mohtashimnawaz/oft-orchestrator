use std::process::Command;
use anyhow::{Result, Context};
use crate::utils;

pub async fn deploy_evm_oft(chain_id: u32, endpoint_address: &str) -> Result<String> {
    println!("🛠️  Spawning Foundry to deploy OFT on chain ID {}...", chain_id);

    // Use a public RPC for Sepolia (or your target chain)
    let rpc_url = "https://ethereum-sepolia-rpc.publicnode.com"; 

    let output = Command::new("forge")
        .current_dir("./evm")
        .arg("script")
        .arg("script/DeployOFT.s.sol:DeployOFT")
        .arg("--sig")
        .arg("run(address)") 
        .arg(endpoint_address)
        .arg("--rpc-url")
        .arg(rpc_url)
        .arg("--broadcast")
        .output()
        .context("Failed to execute forge script")?;

    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;

    // Debug logging to help you see what's happening
    println!("--- FORGE STDOUT ---\n{}", stdout);
    if !stderr.is_empty() {
        println!("--- FORGE STDERR ---\n{}", stderr);
    }
    
    if !output.status.success() {
        anyhow::bail!("Foundry script failed: {}", stderr);
    }

    let address = utils::parse_forge_output(&stdout)
        .context("Could not find DEPLOYED_ADDR in forge output. Check STDOUT above.")?;

    Ok(address)
}

// In src/evm_ops.rs

pub async fn set_peer_evm(oft_addr: &str, target_eid: u32, peer_bytes: String) -> Result<()> {
    println!("🔗 Wiring EVM -> Solana...");
    
    // This executes the 'cast' command using the private key from your .env
    Command::new("cast")
       .arg("send")
       .arg(oft_addr)
       .arg("setPeer(uint32,bytes32)")
       .arg(target_eid.to_string())
       .arg(peer_bytes)
       .arg("--rpc-url").arg("https://ethereum-sepolia-rpc.publicnode.com")
       .arg("--private-key").arg(std::env::var("PRIVATE_KEY")?) // This will now work!
       .output()?;

    Ok(())
}