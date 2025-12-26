use solana_sdk::pubkey::Pubkey;
use hex;

pub fn pad_evm_address(addr_str: &str) -> [u8; 32] {
    let clean_hex = addr_str.trim_start_matches("0x");
    let bytes = hex::decode(clean_hex).expect("Invalid Hex String");
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(&bytes);
    padded
}

pub fn pubkey_to_hex32(pubkey: &Pubkey) -> String {
    let bytes = pubkey.to_bytes();
    format!("0x{}", hex::encode(bytes))
}

pub fn parse_forge_output(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("DEPLOYED_ADDR:") {
            let parts: Vec<&str> = line.split("DEPLOYED_ADDR:").collect();
            if parts.len() > 1 {
                return Some(parts[1].trim().to_string());
            }
        }
    }
    None
}