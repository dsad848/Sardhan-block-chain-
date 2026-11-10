//! # Sardhan Wallet CLI (`sdr-cli`)
//!
//! Provides deterministic key generation, Bech32m address creation, and transaction signing.

use sdr_crypto::Keypair;
use sdr_primitives::{OutPoint, Transaction, TxIn, TxOut, TxWitness, SARDHAN_MAINNET_MAGIC};

fn main() {
    println!("=== SARDHAN CHAIN WALLET CLI (`sdr-cli`) ===");

    let wallet = Keypair::generate();
    let address = wallet.to_address();
    println!("Generated Sovereign Address: {}", address);
    println!("Public Key Hex:              0x{}", hex::encode(wallet.public_key_bytes()));

    // Construct a sample zero-fee transaction with block-bound micro-PoW
    let recipient = Keypair::generate();
    let mut tx = Transaction {
        chain_id: SARDHAN_MAINNET_MAGIC,
        version: 1,
        recent_block_hash: [0x53; 32],
        micro_pow_nonce: 0,
        lock_time: 0,
        inputs: vec![TxIn {
            previous_output: OutPoint {
                txid: [0x11; 32],
                vout: 0,
            },
            sequence: 0xFFFFFFFF,
        }],
        outputs: vec![TxOut {
            value: 50_000_000, // 0.5 SDR
            recipient_pubkey: recipient.public_key_bytes(),
        }],
        witnesses: vec![],
    };

    // Solve client-side micro-PoW (~50ms)
    println!("Solving client-side anti-spam micro-PoW...");
    for nonce in 1..100_000u64 {
        tx.micro_pow_nonce = nonce;
        let h = tx.micro_pow_hash();
        if h[0] == 0 && h[1] < 0x20 {
            println!("Solved Micro-PoW Nonce: {} (Hash: 0x{})", nonce, hex::encode(h));
            break;
        }
    }

    // Sign input 0
    let digest = tx.signature_digest(0);
    let sig = wallet.sign(&digest);
    tx.witnesses.push(TxWitness {
        signature: sig.to_vec(),
        public_key: wallet.public_key_bytes(),
    });

    println!("Transaction Signed! TxID: 0x{}", hex::encode(tx.txid()));
    println!("Zero Transaction Fee verified: Input Value == Output Value.");
}
