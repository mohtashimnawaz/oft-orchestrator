use solana_sdk::{
    signature::{read_keypair_file, Keypair, Signer},
    pubkey::Pubkey,
    system_program,
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
    commitment_config::CommitmentConfig,
};
use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use borsh::BorshSerialize;
use std::str::FromStr;
use anyhow::{Result, Context};

// ⚠️ PASTE THE ID FROM YOUR SCRIPT OUTPUT HERE ⚠️
const LZ_PROGRAM_ID: &str = "DQTTjSLNrNU97djqffEeRKPFD8idj12CiUeXfEg7AHbp"; 

#[derive(BorshSerialize)]
struct InitAdapterArgs {
    shared_decimals: u8,
}

#[derive(BorshSerialize)]
struct SetPeerArgs {
    dst_eid: u32,
    peer_address: [u8; 32],
}

pub async fn init_adapter(mint_str: &str) -> Result<Pubkey> {
    println!("🛠️  Initializing Solana OFT Adapter (Auto) for Mint: {}", mint_str);

    let rpc_url = "https://api.devnet.solana.com";
    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let payer_path = shellexpand::tilde("~/.config/solana/id.json");
    
    // FIX 1: Map error for anyhow
    let payer = read_keypair_file(payer_path.as_ref())
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to find Solana wallet")?;

    let program_id = Pubkey::from_str(LZ_PROGRAM_ID)?;
    let mint = Pubkey::from_str(mint_str)?;

    // SEED: LZAutoV1 (Matches the script)
    let (oft_config_pda, _bump) = Pubkey::find_program_address(
        &[b"LZAutoV1", mint.as_ref()], 
        &program_id
    );
    println!("📍 Calculated PDA: {}", oft_config_pda);

    // Compute discriminator for "init_adapter" dynamically
    let mut hasher = Sha256::new();
    hasher.update("global:init_adapter");
    let hash = hasher.finalize();
    let mut discriminator: [u8; 8] = [0u8; 8];
    discriminator.copy_from_slice(&hash[0..8]);
    println!("🔧 Using discriminator for 'init_adapter': {}", hex::encode(&discriminator));
    let args = InitAdapterArgs { shared_decimals: 6 };
    
    let mut data = Vec::new();
    data.extend_from_slice(&discriminator);
    args.serialize(&mut data)?;

    let accounts = vec![
        AccountMeta::new(oft_config_pda, false),
        AccountMeta::new_readonly(mint, false), 
        AccountMeta::new(payer.pubkey(), true), 
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = Instruction::new_with_bytes(program_id, &data, accounts);

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    println!("🚀 Sending Init Transaction...");
    match client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("✅ Solana Adapter Initialized! Tx: {}", sig),
        Err(e) => {
            println!("⚠️  Init transaction failed: {}", e);
            println!("🔍 Simulating init transaction to fetch program logs...");
            match client.simulate_transaction(&transaction) {
                Ok(sim) => {
                    if let Some(logs) = sim.value.logs {
                        println!("📜 Init program logs:");
                        for l in logs { println!("{}", l); }
                    } else { println!("📜 No logs returned in simulation."); }
                    if let Some(err) = sim.value.err {
                        println!("Simulated init instruction error: {:?}", err);
                    }
                },
                Err(se) => println!("Simulation RPC error: {}", se),
            }
        }
    }

    Ok(oft_config_pda)
}

pub async fn set_peer_solana(oft_config: Pubkey, target_eid: u32, peer_address: [u8; 32]) -> Result<()> {
    println!("🔗 Wiring Solana -> EVM (EID: {})...", target_eid);

    let rpc_url = "https://api.devnet.solana.com";
    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let payer_path = shellexpand::tilde("~/.config/solana/id.json");
    
    // FIX 2: Map error for anyhow (This was the missing piece!)
    let payer = read_keypair_file(payer_path.as_ref())
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to read keypair")?;

    let program_id = Pubkey::from_str(LZ_PROGRAM_ID)?;

    // Compute discriminator for "wire_evm" dynamically (first 8 bytes of sha256('global:wire_evm'))
    let mut hasher = Sha256::new();
    hasher.update("global:wire_evm");
    let hash = hasher.finalize();
    let mut discriminator: [u8; 8] = [0u8; 8];
    discriminator.copy_from_slice(&hash[0..8]);
    println!("🔧 Using discriminator for 'wire_evm': {}", hex::encode(&discriminator)); 

    // Diagnostic: try candidate Anchor discriminators and simulate them so we can find a match without sending on-chain txs
    let candidates = [
        "wire_evm",
        "set_peer",
        "set_peer_solana",
        "set_peer_v1",
        "set_peer_with_peer",
        "initialize",
        "init_adapter",
        "set_peer_evm",
    ];
    println!("🔎 Anchor discriminator hints (name -> first 8 bytes of sha256('global:<name>')) and simulation result:");

    let args = SetPeerArgs {
        dst_eid: target_eid,
        peer_address: peer_address,
    };

    // Fetch a recent blockhash for signed simulation transactions
    let recent_blockhash = client.get_latest_blockhash()?;

    for name in candidates.iter() {
        let mut hasher = Sha256::new();
        hasher.update(format!("global:{}", name));
        let hash = hasher.finalize();
        let disc = &hash[0..8];
        println!("  {} -> {}", name, hex::encode(disc));

        // Build a trial instruction using this discriminator and simulate it
        let mut trial_data = Vec::new();
        trial_data.extend_from_slice(disc);
        args.serialize(&mut trial_data)?;

        let trial_accounts = vec![AccountMeta::new(oft_config, false), AccountMeta::new(payer.pubkey(), true)];
        let trial_inst = Instruction::new_with_bytes(program_id, &trial_data, trial_accounts);

        let trial_tx = Transaction::new_signed_with_payer(
            &[trial_inst],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );

        match client.simulate_transaction(&trial_tx) {
            Ok(sim) => {
                if sim.value.err.is_none() {
                    println!("    ✅ {} simulation returned no error (candidate likely correct). Logs:", name);
                    if let Some(logs) = sim.value.logs {
                        for l in logs { println!("      {}", l); }
                    }
                } else {
                    println!("    ❌ {} simulation error: {:?}", name, sim.value.err);
                }
            },
            Err(se) => {
                println!("    ⚠️ Simulation RPC error for {}: {}", name, se);
            }
        }
    }

    let mut data = Vec::new();
    data.extend_from_slice(&discriminator);
    args.serialize(&mut data)?;

    // Debug: print instruction payload and addresses
    println!("🔧 Instruction data (hex): {}", hex::encode(&data));
    println!("🔧 Peer address (hex): {}", hex::encode(peer_address));
    println!("🔧 OFT config: {}", oft_config);

    let accounts = vec![
        AccountMeta::new(oft_config, false), 
        AccountMeta::new(payer.pubkey(), true), 
    ];

    let instruction = Instruction::new_with_bytes(program_id, &data, accounts);

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    match client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => {
            println!("✅ Solana Peer Set! Tx: {}", sig);
            Ok(())
        },
        Err(e) => {
            println!("❌ Transaction failed: {}", e);
            println!("🔍 Simulating transaction to fetch program logs...");
            match client.simulate_transaction(&transaction) {
                Ok(sim) => {
                    if let Some(logs) = sim.value.logs {
                        println!("📜 Program logs:");
                        for l in logs { println!("{}", l); }
                    } else { println!("📜 No logs returned in simulation."); }
                    if let Some(err) = sim.value.err {
                        println!("Simulated instruction error: {:?}", err);
                    }
                },
                Err(se) => {
                    println!("Simulation RPC error: {}", se);
                }
            }
            return Err(anyhow::anyhow!(format!("Failed to send tx: {}", e)));
        }
    }
}