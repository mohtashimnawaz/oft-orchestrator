use solana_sdk::{
    signature::{read_keypair_file, Keypair, Signer},
    pubkey::Pubkey,
    system_program,
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
    commitment_config::CommitmentConfig,
};
use solana_client::rpc_client::RpcClient;
use borsh::BorshSerialize;
use std::str::FromStr;
use anyhow::{Result, Context};

// ✅ CONFIGURATION: Your deployed Anchor Program ID
const LZ_PROGRAM_ID: &str = "FX9LD6JPbSim1h7m7pJxKXdCB9SUL4SMbdyWnwtHB6LQ";// ------------------------------------------------------------------
// DATA STRUCTURES
// ------------------------------------------------------------------

#[derive(BorshSerialize)]
struct InitAdapterArgs {
    shared_decimals: u8,
}

#[derive(BorshSerialize)]
struct SetPeerArgs {
    dst_eid: u32,
    peer_address: [u8; 32],
}

// ------------------------------------------------------------------
// LOGIC
// ------------------------------------------------------------------

pub async fn init_adapter(mint_str: &str) -> Result<Pubkey> {
    println!("🛠️  Initializing Solana OFT Adapter for Mint: {}", mint_str);

    let rpc_url = "https://api.devnet.solana.com";
    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    
    // 1. Load Wallet (with error mapping fix)
    let payer_path = shellexpand::tilde("~/.config/solana/id.json");
    let payer = read_keypair_file(payer_path.as_ref())
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to find Solana wallet")?;

    let program_id = Pubkey::from_str(LZ_PROGRAM_ID)?;
    let mint = Pubkey::from_str(mint_str)?;

    // 2. Derive PDA
    let (oft_config_pda, _bump) = Pubkey::find_program_address(
        &[b"OftConfig", mint.as_ref()], 
        &program_id
    );
    println!("📍 Calculated OFT Config PDA: {}", oft_config_pda);

    // 3. Construct Instruction: "init_adapter"
    // Discriminator: sha256("global:init_adapter")[..8]
    let discriminator: [u8; 8] = [207, 39, 175, 126, 226, 117, 161, 149]; 
    let args = InitAdapterArgs { shared_decimals: 6 };
    
    let mut instruction_data = Vec::new();
    instruction_data.extend_from_slice(&discriminator);
    args.serialize(&mut instruction_data)?;

    let accounts = vec![
        AccountMeta::new(oft_config_pda, false),
        AccountMeta::new_readonly(mint, false), 
        AccountMeta::new(payer.pubkey(), true), 
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = Instruction::new_with_bytes(program_id, &instruction_data, accounts);

    // 4. Send Transaction
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
            // Gracefully handle "Account already initialized" (0x65 / 101)
            if e.to_string().contains("0x65") || e.to_string().contains("custom program error: 0x1") {
                println!("⚠️  Account already initialized (skipping init). Continuing...");
            } else {
                println!("⚠️  Transaction failed: {}", e);
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
    let payer = read_keypair_file(payer_path.as_ref())
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to read keypair")?;

    let program_id = Pubkey::from_str(LZ_PROGRAM_ID)?;

    // 1. Construct Instruction: "set_peer"
    // Discriminator: sha256("global:set_peer")[..8]
    // ⚠️ IMPORTANT: This must be different from init_adapter!
    let discriminator: [u8; 8] = [105, 17, 237, 72, 196, 21, 198, 118]; 

    let args = SetPeerArgs {
        dst_eid: target_eid,
        peer_address: peer_address,
    };

    let mut instruction_data = Vec::new();
    instruction_data.extend_from_slice(&discriminator);
    args.serialize(&mut instruction_data)?;

    // 2. Accounts: [oft_config, admin]
    let accounts = vec![
        AccountMeta::new(oft_config, false), 
        AccountMeta::new(payer.pubkey(), true), 
    ];

    let instruction = Instruction::new_with_bytes(program_id, &instruction_data, accounts);

    // 3. Send Transaction
    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let signature = client.send_and_confirm_transaction(&transaction)?;
    println!("✅ Solana Peer Set! Tx: {}", signature);

    Ok(())
}