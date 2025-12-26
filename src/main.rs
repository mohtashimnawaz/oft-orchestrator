use clap::{Parser, Subcommand};
use anyhow::Result;
use dotenv::dotenv;
use std::path::Path;

mod evm_ops;
mod solana_ops;
mod utils;

#[derive(Parser)]
#[command(name = "oft-cli")]
#[command(about = "Orchestrates LayerZero V2 OFT Deployment", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Deploy {
        #[arg(short, long)]
        mint: String,
        #[arg(short, long)]
        evm_chain_id: u32,
        #[arg(short, long)]
        lz_endpoint: String,
        #[arg(long)]
        target_eid: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from the EVM folder
    let env_path = Path::new("evm/.env");
    if env_path.exists() {
        dotenv::from_path(env_path).ok();
    } else {
        println!("⚠️  Warning: evm/.env file not found. Rust might not see PRIVATE_KEY.");
    }

    let cli = Cli::parse();

    match &cli.command {
        Commands::Deploy { mint, evm_chain_id, lz_endpoint, target_eid } => {
            // 1. Setup Solana Side
            let sol_oft_pda = solana_ops::init_adapter(mint).await?;
            
            // 2. Setup EVM Side
            let evm_oft_addr = evm_ops::deploy_evm_oft(*evm_chain_id, lz_endpoint).await?;
            println!("📝 Captured EVM Address: {}", evm_oft_addr);

            // 3. Wire: Solana -> EVM
            let evm_bytes32 = utils::pad_evm_address(&evm_oft_addr);
            solana_ops::set_peer_solana(sol_oft_pda, *target_eid, evm_bytes32).await?;

            // 4. Wire: EVM -> Solana
            let sol_bytes32_hex = utils::pubkey_to_hex32(&sol_oft_pda);
            evm_ops::set_peer_evm(&evm_oft_addr, *target_eid, sol_bytes32_hex).await?;
            
            println!("🚀 Cross-chain setup complete!");
        }
    }
    Ok(())
}