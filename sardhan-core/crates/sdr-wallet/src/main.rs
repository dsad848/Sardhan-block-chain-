//! # Sardhan Native Wallet CLI (`sdr-wallet`)
//!
//! Features:
//! - Local-only Ed25519 key generation & mnemonic backup/recovery (keys never transmitted)
//! - Network-separated Bech32m addresses ('sdr1' Mainnet / 'tsdr1' Testnet / 'rsdr1' Regtest)
//! - Zero-float exact integer SDR -> Sardhan monetary parser
//! - Local transaction building, dynamic micro-PoW solving, and Ed25519 signing
//! - ISO-18004 Version 4-L Reed-Solomon QR code rendering for Bech32m addresses
//! - HardwareSigner trait abstraction for cold/hardware key custody

use sdr_consensus::hash_less_than_target;
use sdr_crypto::{
    decode_address_for_hrp, derive_child_secret, derive_seed_from_mnemonic,
    encode_address_for_hrp, generate_mnemonic, HardwareSigner, Keypair, SoftwareSigner,
};
use sdr_primitives::{
    Network, OutPoint, Transaction, TxIn, TxOut, TxWitness, MIN_TRANSACTION_OUTPUT_VALUE,
    SARDHANS_PER_SDR,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFile {
    pub version: u32,
    pub network: String,
    pub hrp: String,
    pub chain_id: u32,
    pub mnemonic: String,
    pub master_seed_hex: String,
    pub account: u32,
    pub index: u32,
    pub address: String,
    pub public_key_hex: String,
}

impl WalletFile {
    pub fn from_mnemonic(mnemonic: String, passphrase: &str, network: Network) -> Result<Self, String> {
        let cfg = network.config();
        let master_seed = derive_seed_from_mnemonic(&mnemonic, passphrase)
            .map_err(|e| format!("Invalid mnemonic: {e}"))?;
        let signer = SoftwareSigner { master_seed };
        let pubkey = signer
            .get_public_key(0, 0)
            .map_err(|e| format!("Key derivation failed: {e}"))?;
        let addr = encode_address_for_hrp(cfg.address_hrp, &pubkey)
            .map_err(|e| format!("Address encoding failed: {e}"))?;

        Ok(Self {
            version: 2,
            network: cfg.name.to_string(),
            hrp: cfg.address_hrp.to_string(),
            chain_id: cfg.chain_id,
            mnemonic,
            master_seed_hex: hex::encode(master_seed),
            account: 0,
            index: 0,
            address: addr,
            public_key_hex: hex::encode(pubkey),
        })
    }

    pub fn signer(&self) -> Result<SoftwareSigner, String> {
        let bytes = hex::decode(&self.master_seed_hex).map_err(|e| e.to_string())?;
        let master_seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "Invalid master seed length".to_string())?;
        Ok(SoftwareSigner { master_seed })
    }

    pub fn keypair(&self) -> Result<Keypair, String> {
        let signer = self.signer()?;
        let child = derive_child_secret(&signer.master_seed, self.account, self.index);
        Ok(Keypair::from_secret_bytes(&child))
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    pub fn load(path: &PathBuf) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|e| format!("Failed reading wallet file {:?}: {e}", path))?;
        serde_json::from_str(&raw).map_err(|e| format!("Corrupt wallet JSON: {e}"))
    }
}

/// Hardware wallet device stub representing unattached USB HID hardware bridge
pub struct UsbHardwareWallet;

impl HardwareSigner for UsbHardwareWallet {
    fn device_name(&self) -> &str {
        "USB-HID Cold Hardware Signer"
    }

    fn get_public_key(&self, _account: u32, _index: u32) -> Result<[u8; 32], sdr_crypto::CryptoError> {
        Err(sdr_crypto::CryptoError::HardwareError(
            "No physical USB-HID hardware wallet detected on host".into(),
        ))
    }

    fn sign_digest(
        &self,
        _account: u32,
        _index: u32,
        _digest: &[u8; 32],
    ) -> Result<[u8; 64], sdr_crypto::CryptoError> {
        Err(sdr_crypto::CryptoError::HardwareError(
            "No physical USB-HID hardware wallet detected on host".into(),
        ))
    }
}

/// Parse decimal SDR string (e.g. "12.50") strictly using integer math (NO floats!)
pub fn parse_sdr_to_sardhans(amount_str: &str) -> Result<u64, String> {
    let trimmed = amount_str.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return Err("Amount must be positive".into());
    }
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() > 2 {
        return Err("Invalid decimal amount format".into());
    }
    let whole: u64 = if parts[0].is_empty() {
        0
    } else {
        parts[0]
            .parse::<u64>()
            .map_err(|_| "Invalid whole SDR digits".to_string())?
    };
    let frac: u64 = if parts.len() == 2 {
        let f = parts[1];
        if f.len() > 8 {
            return Err("Precision exceeds 8 decimal places (1 Sardhan = 1e-8 SDR)".into());
        }
        let padded = format!("{f:0<8}");
        padded
            .parse::<u64>()
            .map_err(|_| "Invalid fractional SDR digits".to_string())?
    } else {
        0
    };
    whole
        .checked_mul(SARDHANS_PER_SDR)
        .and_then(|w| w.checked_add(frac))
        .ok_or_else(|| "Amount overflow".to_string())
}

fn rpc_call(host_port: &str, method: &str, params: Value) -> Result<Value, String> {
    let mut stream = TcpStream::connect(host_port)
        .map_err(|e| format!("RPC connect error ({host_port}): {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    })
    .to_string();
    let req = format!(
        "POST / HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("RPC write error: {e}"))?;
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .map_err(|e| format!("RPC read error: {e}"))?;
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .ok_or_else(|| "Malformed HTTP response".to_string())?;
    let parsed: Value =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON-RPC response: {e}"))?;
    if let Some(err) = parsed.get("error") {
        if !err.is_null() {
            return Err(format!("RPC error: {err}"));
        }
    }
    parsed
        .get("result")
        .cloned()
        .ok_or_else(|| "Missing JSON-RPC result".to_string())
}

/// Real ISO-18004 QR Code Encoder (Version 4-L, 33x33 modules, Byte mode, up to 78 bytes, GF(256) ECC)
pub fn encode_qr_version4_l(text: &str) -> Option<Vec<Vec<bool>>> {
    let data = text.as_bytes();
    if data.len() > 78 {
        return None;
    }

    // Build bit stream: Mode 0100 (4b) + CharCount (8b) + payload + terminator + padding
    let mut bits = Vec::with_capacity(80 * 8);
    let push_bits = |buf: &mut Vec<bool>, val: usize, count: usize| {
        for i in (0..count).rev() {
            buf.push(((val >> i) & 1) == 1);
        }
    };
    push_bits(&mut bits, 0b0100, 4);
    push_bits(&mut bits, data.len(), 8);
    for &b in data {
        push_bits(&mut bits, b as usize, 8);
    }
    let total_data_bits = 80 * 8; // Version 4-L has 80 data codewords
    let term_len = (total_data_bits - bits.len()).min(4);
    for _ in 0..term_len {
        bits.push(false);
    }
    while bits.len() % 8 != 0 {
        bits.push(false);
    }
    let mut codewords = Vec::with_capacity(100);
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for &bit in chunk {
            byte = (byte << 1) | (bit as u8);
        }
        codewords.push(byte);
    }
    let pad_bytes = [0xECu8, 0x11u8];
    let mut p_idx = 0;
    while codewords.len() < 80 {
        codewords.push(pad_bytes[p_idx % 2]);
        p_idx += 1;
    }

    // GF(256) Reed-Solomon with 20 EC codewords for Version 4-L (100 total codewords)
    let ecc = rs_encode(&codewords, 20);
    codewords.extend_from_slice(&ecc);

    let size = 33usize;
    let mut grid = vec![vec![false; size]; size];
    let mut reserved = vec![vec![false; size]; size];

    let place_finder = |g: &mut [Vec<bool>], r: &mut [Vec<bool>], row: usize, col: usize| {
        for dy in 0..7 {
            for dx in 0..7 {
                let dark = dy == 0
                    || dy == 6
                    || dx == 0
                    || dx == 6
                    || (2..=4).contains(&dy) && (2..=4).contains(&dx);
                g[row + dy][col + dx] = dark;
                r[row + dy][col + dx] = true;
            }
        }
    };
    place_finder(&mut grid, &mut reserved, 0, 0);
    place_finder(&mut grid, &mut reserved, 0, size - 7);
    place_finder(&mut grid, &mut reserved, size - 7, 0);

    // Separators
    for i in 0..8 {
        reserved[7][i] = true;
        reserved[i][7] = true;
        reserved[7][size - 1 - i] = true;
        reserved[i][size - 8] = true;
        reserved[size - 8][i] = true;
        reserved[size - 1 - i][7] = true;
    }

    // Timing patterns
    for i in 8..(size - 8) {
        let dark = i % 2 == 0;
        grid[6][i] = dark;
        reserved[6][i] = true;
        grid[i][6] = dark;
        reserved[i][6] = true;
    }

    // Version 4 alignment pattern centered at (26, 26)
    for dy in 0..5usize {
        for dx in 0..5usize {
            let y = 24 + dy;
            let x = 24 + dx;
            let dark = dy == 0 || dy == 4 || dx == 0 || dx == 4 || (dy == 2 && dx == 2);
            grid[y][x] = dark;
            reserved[y][x] = true;
        }
    }

    // Dark module + reserve format info strips
    grid[size - 8][8] = true;
    reserved[size - 8][8] = true;
    for i in 0..9 {
        if i != 6 {
            reserved[8][i] = true;
            reserved[i][8] = true;
        }
    }
    for i in 0..8 {
        reserved[8][size - 1 - i] = true;
        reserved[size - 1 - i][8] = true;
    }

    // Place data bits in zig-zag columns using Mask 0 ((r + c) % 2 == 0)
    let mut bit_stream = Vec::with_capacity(codewords.len() * 8);
    for cw in codewords {
        for b in (0..8).rev() {
            bit_stream.push(((cw >> b) & 1) == 1);
        }
    }
    let mut bit_idx = 0usize;
    let mut col = size as isize - 1;
    let mut upward = true;
    while col > 0 {
        if col == 6 {
            col -= 1;
        }
        for step in 0..size {
            let r = if upward { size - 1 - step } else { step };
            for dx in 0..2 {
                let c = (col - dx) as usize;
                if !reserved[r][c] {
                    let raw_bit = if bit_idx < bit_stream.len() {
                        bit_stream[bit_idx]
                    } else {
                        false
                    };
                    bit_idx += 1;
                    let mask = (r + c) % 2 == 0;
                    grid[r][c] = raw_bit ^ mask;
                }
            }
        }
        upward = !upward;
        col -= 2;
    }

    // Write format info for ECL=L (01), Mask=0 (000) -> masked BCH bits = 0x77C4 (MSB first)
    let fmt = 0x77C4u16;
    for i in 0..15 {
        let dark = ((fmt >> (14 - i)) & 1) == 1;
        // Strip A (around top-left)
        if i < 6 {
            grid[8][i] = dark;
        } else if i == 6 {
            grid[8][7] = dark;
        } else if i == 7 {
            grid[8][8] = dark;
        } else if i == 8 {
            grid[7][8] = dark;
        } else {
            grid[14 - i][8] = dark;
        }
        // Strip B (bottom-left & top-right)
        if i < 7 {
            grid[size - 1 - i][8] = dark;
        } else {
            grid[8][size - 15 + i] = dark;
        }
    }

    Some(grid)
}

fn rs_encode(data: &[u8], ec_len: usize) -> Vec<u8> {
    let mut exp = [0u8; 512];
    let mut log = [0u8; 256];
    let mut x = 1u16;
    for i in 0..255 {
        exp[i] = x as u8;
        log[x as usize] = i as u8;
        x <<= 1;
        if (x & 0x100) != 0 {
            x ^= 0x11D;
        }
    }
    for i in 255..512 {
        exp[i] = exp[i - 255];
    }
    let gf_mul = |a: u8, b: u8| -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            exp[(log[a as usize] as usize) + (log[b as usize] as usize)]
        }
    };

    let mut gen = vec![1u8];
    for i in 0..ec_len {
        let mut next = vec![0u8; gen.len() + 1];
        for j in 0..gen.len() {
            next[j] ^= gen[j];
            next[j + 1] ^= gf_mul(gen[j], exp[i]);
        }
        gen = next;
    }

    let mut msg = vec![0u8; data.len() + ec_len];
    msg[..data.len()].copy_from_slice(data);
    for i in 0..data.len() {
        let coef = msg[i];
        if coef != 0 {
            for j in 0..gen.len() {
                msg[i + j] ^= gf_mul(gen[j], coef);
            }
        }
    }
    msg[data.len()..].to_vec()
}

pub fn render_qr_ascii(text: &str) -> String {
    let Some(grid) = encode_qr_version4_l(text) else {
        return format!("[QR too long for v4: {text}]");
    };
    let n = grid.len();
    let mut out = String::new();
    let blank_row = vec![false; n + 4];
    let mut padded = Vec::with_capacity(n + 4);
    padded.push(blank_row.clone());
    padded.push(blank_row.clone());
    for row in &grid {
        let mut r = vec![false, false];
        r.extend_from_slice(row);
        r.push(false);
        r.push(false);
        padded.push(r);
    }
    padded.push(blank_row.clone());
    padded.push(blank_row);

    let mut y = 0;
    while y < padded.len() {
        for x in 0..padded[y].len() {
            let top = padded[y][x];
            let bot = if y + 1 < padded.len() { padded[y + 1][x] } else { false };
            let ch = match (top, bot) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            };
            out.push(ch);
        }
        out.push('\n');
        y += 2;
    }
    out
}

fn print_help() {
    println!(
        "Sardhan Native Wallet CLI (`sdr-wallet`)
Usage: sdr-wallet [--wallet <path>] [--network <testnet|mainnet|regtest>] [--rpc <host:port>] <COMMAND>
Commands:
  create [--words 18|24] [--passphrase <str>]    Generate new mnemonic wallet & save locally
  recover --mnemonic \"...\" [--passphrase <str>] Recover wallet from mnemonic seed phrase
  address                                        Show Bech32m address & terminal QR code
  balance                                        Query confirmed/spendable SDR balance & UTXOs via RPC
  history                                        Query transaction history for this address via RPC
  send --to <addr> --amount <SDR>                Build locally, solve micro-PoW, sign locally & broadcast zero-fee tx
  qr [address]                                   Render ISO-18004 QR code for an address
  hw-status                                      Inspect HardwareSigner device bridge status"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    let mut wallet_path = PathBuf::from("./wallet.json");
    let mut network = Network::Testnet;
    let mut rpc_url = "127.0.0.1:19332".to_string();
    let mut rpc_overridden = false;
    let mut cmd_idx = 1usize;

    while cmd_idx < args.len() && args[cmd_idx].starts_with("--") {
        match args[cmd_idx].as_str() {
            "--wallet" if cmd_idx + 1 < args.len() => {
                wallet_path = PathBuf::from(&args[cmd_idx + 1]);
                cmd_idx += 2;
            }
            "--network" if cmd_idx + 1 < args.len() => {
                network = Network::from_str_name(&args[cmd_idx + 1]).unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
                cmd_idx += 2;
            }
            "--rpc" if cmd_idx + 1 < args.len() => {
                rpc_url = args[cmd_idx + 1]
                    .trim_start_matches("http://")
                    .trim_end_matches('/')
                    .to_string();
                rpc_overridden = true;
                cmd_idx += 2;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            other => {
                eprintln!("Unknown global option: {other}");
                std::process::exit(1);
            }
        }
    }

    if !rpc_overridden {
        rpc_url = format!("127.0.0.1:{}", network.config().default_rpc_port);
    }

    if cmd_idx >= args.len() {
        print_help();
        return;
    }

    let subcmd = args[cmd_idx].as_str();
    let rest = &args[cmd_idx + 1..];

    match subcmd {
        "create" => {
            let mut words = 18usize;
            let mut passphrase = String::new();
            let mut j = 0;
            while j < rest.len() {
                match rest[j].as_str() {
                    "--words" if j + 1 < rest.len() => {
                        words = rest[j + 1].parse().unwrap_or(18);
                        j += 2;
                    }
                    "--passphrase" if j + 1 < rest.len() => {
                        passphrase = rest[j + 1].clone();
                        j += 2;
                    }
                    _ => j += 1,
                }
            }
            let mnemonic = generate_mnemonic(words).unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            let w = WalletFile::from_mnemonic(mnemonic.clone(), &passphrase, network)
                .unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
            w.save(&wallet_path).unwrap_or_else(|e| {
                eprintln!("Error saving wallet: {e}");
                std::process::exit(1);
            });
            println!("=== NEW SARDHAN WALLET CREATED (LOCAL CUSTODY ONLY) ===");
            println!("Network:      {} (ChainID: 0x{:08x})", w.network, w.chain_id);
            println!("Wallet File:  {:?}", wallet_path);
            println!("Address:      {}", w.address);
            println!("Public Key:   0x{}", w.public_key_hex);
            println!("Seed Phrase ({words} words) — STORE OFFLINE, NEVER SHARE:");
            println!("  {}", mnemonic);
            println!("\nReceive QR Code:\n{}", render_qr_ascii(&w.address));
        }
        "recover" => {
            let mut mnemonic = None;
            let mut passphrase = String::new();
            let mut j = 0;
            while j < rest.len() {
                match rest[j].as_str() {
                    "--mnemonic" if j + 1 < rest.len() => {
                        mnemonic = Some(rest[j + 1].clone());
                        j += 2;
                    }
                    "--passphrase" if j + 1 < rest.len() => {
                        passphrase = rest[j + 1].clone();
                        j += 2;
                    }
                    _ => j += 1,
                }
            }
            let phrase = mnemonic.unwrap_or_else(|| {
                eprintln!("Error: --mnemonic \"...\" is required");
                std::process::exit(1);
            });
            let w = WalletFile::from_mnemonic(phrase, &passphrase, network).unwrap_or_else(|e| {
                eprintln!("Recovery failed: {e}");
                std::process::exit(1);
            });
            w.save(&wallet_path).unwrap_or_else(|e| {
                eprintln!("Error saving recovered wallet: {e}");
                std::process::exit(1);
            });
            println!("Wallet recovered successfully to {:?}", wallet_path);
            println!("Address: {} (Network: {})", w.address, w.network);
        }
        "address" => {
            let w = WalletFile::load(&wallet_path).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            println!("Address:    {}", w.address);
            println!("Network:    {}", w.network);
            println!("Public Key: 0x{}", w.public_key_hex);
            println!("\n{}", render_qr_ascii(&w.address));
        }
        "qr" => {
            let target = if !rest.is_empty() {
                rest[0].clone()
            } else {
                WalletFile::load(&wallet_path)
                    .unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    })
                    .address
            };
            println!("QR Code for {target}:\n{}", render_qr_ascii(&target));
        }
        "balance" => {
            let w = WalletFile::load(&wallet_path).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let res = rpc_call(&rpc_url, "getaddressinfo", json!([w.address])).unwrap_or_else(|e| {
                eprintln!("RPC query failed: {e}");
                std::process::exit(1);
            });
            println!("Address:          {}", w.address);
            println!("Network:          {}", res["network"]);
            println!(
                "Total Balance:    {:.8} SDR ({} Sardhans)",
                res["balance_sdr"].as_f64().unwrap_or(0.0),
                res["balance_sardhans"].as_u64().unwrap_or(0)
            );
            println!(
                "Spendable:        {:.8} SDR",
                res["spendable_sdr"].as_f64().unwrap_or(0.0)
            );
            println!(
                "UTXOs:            {}",
                res["utxos"].as_array().map(|a| a.len()).unwrap_or(0)
            );
        }
        "history" => {
            let w = WalletFile::load(&wallet_path).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let res = rpc_call(&rpc_url, "getaddressinfo", json!([w.address])).unwrap_or_else(|e| {
                eprintln!("RPC query failed: {e}");
                std::process::exit(1);
            });
            let hist = res["history"].as_array().cloned().unwrap_or_default();
            println!("Transaction History for {} ({} entries):", w.address, hist.len());
            for item in hist {
                println!(
                    "  Block #{:<6} | {:<9} | {:>14.8} SDR | txid 0x{}",
                    item["block_height"].as_u64().unwrap_or(0),
                    item["direction"].as_str().unwrap_or(""),
                    (item["amount_sardhans"].as_u64().unwrap_or(0) as f64) / 1e8,
                    item["txid"].as_str().unwrap_or("")
                );
            }
        }
        "send" => {
            let mut to_addr = None;
            let mut amount_str = None;
            let mut j = 0;
            while j < rest.len() {
                match rest[j].as_str() {
                    "--to" if j + 1 < rest.len() => {
                        to_addr = Some(rest[j + 1].clone());
                        j += 2;
                    }
                    "--amount" if j + 1 < rest.len() => {
                        amount_str = Some(rest[j + 1].clone());
                        j += 2;
                    }
                    _ => j += 1,
                }
            }
            let to = to_addr.unwrap_or_else(|| {
                eprintln!("--to <address> required");
                std::process::exit(1);
            });
            let amt_sardhans = parse_sdr_to_sardhans(
                &amount_str.unwrap_or_else(|| {
                    eprintln!("--amount <SDR> required");
                    std::process::exit(1);
                }),
            )
            .unwrap_or_else(|e| {
                eprintln!("Invalid amount: {e}");
                std::process::exit(1);
            });
            if amt_sardhans < MIN_TRANSACTION_OUTPUT_VALUE {
                eprintln!("Amount below dust limit ({MIN_TRANSACTION_OUTPUT_VALUE} Sardhans)");
                std::process::exit(1);
            }

            let w = WalletFile::load(&wallet_path).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let recipient_pk = decode_address_for_hrp(&to, &w.hrp).unwrap_or_else(|e| {
                eprintln!("Address/Network separation violation: {e}");
                std::process::exit(1);
            });
            let sender_pk = decode_address_for_hrp(&w.address, &w.hrp).unwrap();

            // Fetch spendable UTXOs & recent block hash from RPC
            let addr_info = rpc_call(&rpc_url, "getaddressinfo", json!([w.address])).unwrap_or_else(|e| {
                eprintln!("Failed querying UTXOs from {rpc_url}: {e}");
                std::process::exit(1);
            });
            let mining_info = rpc_call(&rpc_url, "getmininginfo", json!([])).unwrap_or_else(|e| {
                eprintln!("Failed querying chain tip from {rpc_url}: {e}");
                std::process::exit(1);
            });

            let recent_hex = mining_info["recent_block_hashes"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    eprintln!("RPC missing recent_block_hashes");
                    std::process::exit(1);
                });
            let recent_bytes = hex::decode(recent_hex).unwrap();
            let mut recent_block_hash = [0u8; 32];
            recent_block_hash.copy_from_slice(&recent_bytes);

            let target_hex = mining_info["micro_pow_target_hex"].as_str().unwrap_or("");
            let target_bytes: [u8; 32] = hex::decode(target_hex)
                .unwrap()
                .try_into()
                .expect("32-byte micro-PoW target");

            let utxos = addr_info["utxos"].as_array().cloned().unwrap_or_default();
            let mut selected_inputs = Vec::new();
            let mut total_in = 0u64;
            for u in utxos {
                let val = u["value_sardhans"].as_u64().unwrap_or(0);
                let txid_hex = u["txid"].as_str().unwrap_or("");
                let vout = u["vout"].as_u64().unwrap_or(0) as u32;
                let txid_bytes: [u8; 32] = hex::decode(txid_hex).unwrap().try_into().unwrap();
                selected_inputs.push(TxIn {
                    previous_output: OutPoint {
                        txid: txid_bytes,
                        vout,
                    },
                    sequence: 0xffffffff,
                });
                total_in += val;
                if total_in >= amt_sardhans {
                    let change = total_in - amt_sardhans;
                    if change == 0 || change >= MIN_TRANSACTION_OUTPUT_VALUE {
                        break;
                    }
                }
            }

            if total_in < amt_sardhans {
                eprintln!(
                    "Insufficient spendable balance: need {} Sardhans, available {}",
                    amt_sardhans, total_in
                );
                std::process::exit(1);
            }

            let change = total_in - amt_sardhans;
            let mut outputs = vec![TxOut {
                value: amt_sardhans,
                recipient_pubkey: recipient_pk,
            }];
            if change > 0 {
                if change < MIN_TRANSACTION_OUTPUT_VALUE {
                    eprintln!("Change output ({change} Sardhans) below dust limit");
                    std::process::exit(1);
                }
                outputs.push(TxOut {
                    value: change,
                    recipient_pubkey: sender_pk,
                });
            }

            let mut tx = Transaction {
                chain_id: w.chain_id,
                version: 1,
                recent_block_hash,
                micro_pow_nonce: 0,
                lock_time: 0,
                inputs: selected_inputs,
                outputs,
                witnesses: vec![],
            };

            // Solve client-side anti-spam micro-PoW locally
            let mut solved = false;
            for nonce in 1..=2_000_000u64 {
                tx.micro_pow_nonce = nonce;
                if hash_less_than_target(&tx.micro_pow_hash(), &target_bytes) {
                    solved = true;
                    break;
                }
            }
            if !solved {
                eprintln!("Failed finding micro-PoW nonce within 2M iterations");
                std::process::exit(1);
            }

            // Sign locally (private key never leaves this device)
            let signer = w.signer().unwrap();
            let pubkey = signer.get_public_key(w.account, w.index).unwrap();
            for inp_idx in 0..tx.inputs.len() {
                let digest = tx.signature_digest(inp_idx);
                let sig = signer.sign_digest(w.account, w.index, &digest).unwrap();
                tx.witnesses.push(TxWitness {
                    signature: sig.to_vec(),
                    public_key: pubkey,
                });
            }

            let raw_hex = hex::encode(tx.to_bytes());
            let resp = rpc_call(&rpc_url, "sendrawtransaction", json!([raw_hex])).unwrap_or_else(|e| {
                eprintln!("Broadcast failed: {e}");
                std::process::exit(1);
            });
            println!("Zero-Fee Transaction Signed Locally & Broadcast!");
            println!("  TxID:           0x{}", resp["txid"].as_str().unwrap_or(""));
            println!("  Sent:           {:.8} SDR", (amt_sardhans as f64) / 1e8);
            println!("  Fee:            0.00000000 SDR");
            println!("  Micro-PoW Nonce: {}", tx.micro_pow_nonce);
        }
        "hw-status" => {
            let usb = UsbHardwareWallet;
            println!("Hardware Bridge: {}", usb.device_name());
            match usb.get_public_key(0, 0) {
                Ok(pk) => println!("Connected device pubkey: 0x{}", hex::encode(pk)),
                Err(e) => println!("Device status: {e}"),
            }
        }
        _ => {
            eprintln!("Unknown command '{subcmd}'");
            print_help();
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_mnemonic_recovery_and_network_separation() {
        let mnemonic = generate_mnemonic(18).unwrap();
        let w_test = WalletFile::from_mnemonic(mnemonic.clone(), "", Network::Testnet).unwrap();
        let w_main = WalletFile::from_mnemonic(mnemonic, "", Network::Mainnet).unwrap();

        assert!(w_test.address.starts_with("tsdr1"));
        assert!(w_main.address.starts_with("sdr1"));
        assert_eq!(w_test.public_key_hex, w_main.public_key_hex);
        assert!(decode_address_for_hrp(&w_main.address, "tsdr").is_err());
    }

    #[test]
    fn test_integer_amount_parsing() {
        assert_eq!(parse_sdr_to_sardhans("1").unwrap(), 100_000_000);
        assert_eq!(parse_sdr_to_sardhans("0.00001000").unwrap(), 1000);
        assert_eq!(parse_sdr_to_sardhans("25.12345678").unwrap(), 2_512_345_678);
        assert!(parse_sdr_to_sardhans("-5").is_err());
        assert!(parse_sdr_to_sardhans("1.123456789").is_err());
    }

    #[test]
    fn test_qr_encoder_renders_matrix() {
        let kp = Keypair::generate();
        let addr = kp.to_testnet_address();
        let grid = encode_qr_version4_l(&addr).expect("QR v4 encodes 63-char address");
        assert_eq!(grid.len(), 33);
        let ascii = render_qr_ascii(&addr);
        assert!(ascii.contains('█'));
    }
}
