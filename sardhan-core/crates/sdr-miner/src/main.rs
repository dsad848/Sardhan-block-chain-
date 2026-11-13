//! # Sardhan CPU/ARM64 Miner Application (`sdr-miner`)
//!
//! Features:
//! - Real multi-threaded SardhanHash nonce searching against live `sdrd` JSON-RPC (`getblocktemplate` / `submitblock`)
//! - ARM64 (aarch64) & x86_64 architecture reporting
//! - Thermal protection (/sys/class/thermal/thermal_zone*/temp)
//! - Battery protection (/sys/class/power_supply/BAT*/capacity & status)
//! - Configurable `--threads` & `--cpu-limit` duty-cycle throttling
//! - Pause & resume via `--pause-file` or interactive `p` / `r` commands
//! - Honest `--benchmark` comparing SardhanHash vs RandomX-style & VerusHash-style CPU workloads

use sdr_consensus::{compact_to_target, hash_less_than_target};
use sdr_crypto::{hash_blake3, sardhan_hash};
use sdr_primitives::Block;
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct MinerConfig {
    pub rpc_url: String,
    pub coinbase_address: Option<String>,
    pub threads: usize,
    pub cpu_limit_pct: u8,
    pub max_temp_c: f32,
    pub min_battery_pct: u8,
    pub pause_file: Option<PathBuf>,
    pub benchmark_secs: Option<u64>,
    pub max_blocks: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct HardwareHealth {
    pub max_temp_c: Option<f32>,
    pub battery_pct: Option<u8>,
    pub on_ac_power: bool,
}

impl HardwareHealth {
    pub fn sample() -> Self {
        Self {
            max_temp_c: read_max_thermal_celsius(),
            battery_pct: read_battery_capacity_pct(),
            on_ac_power: is_ac_online_or_desktop(),
        }
    }

    pub fn should_throttle(&self, max_temp_c: f32, min_battery_pct: u8) -> Option<String> {
        if let Some(t) = self.max_temp_c {
            if t >= max_temp_c {
                return Some(format!("Thermal limit reached: {t:.1}°C >= {max_temp_c:.1}°C"));
            }
        }
        if !self.on_ac_power {
            if let Some(pct) = self.battery_pct {
                if pct < min_battery_pct {
                    return Some(format!(
                        "Battery protection triggered: {pct}% < {min_battery_pct}% while discharging"
                    ));
                }
            }
        }
        None
    }
}

pub fn read_max_thermal_celsius() -> Option<f32> {
    let Ok(entries) = fs::read_dir("/sys/class/thermal") else {
        return None;
    };
    let mut max_temp: Option<f32> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let temp_path = entry.path().join("temp");
        if let Ok(raw) = fs::read_to_string(temp_path) {
            if let Ok(milli_c) = raw.trim().parse::<f32>() {
                let c = if milli_c > 200.0 { milli_c / 1000.0 } else { milli_c };
                if c > 0.0 && c < 150.0 {
                    max_temp = Some(max_temp.map_or(c, |m| m.max(c)));
                }
            }
        }
    }
    max_temp
}

pub fn read_battery_capacity_pct() -> Option<u8> {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return None;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("BAT") {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(entry.path().join("capacity")) {
            if let Ok(pct) = raw.trim().parse::<u8>() {
                return Some(pct.min(100));
            }
        }
    }
    None
}

pub fn is_ac_online_or_desktop() -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return true;
    };
    let mut found_battery = false;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("BAT") {
            found_battery = true;
            if let Ok(status) = fs::read_to_string(entry.path().join("status")) {
                let s = status.trim().to_ascii_lowercase();
                if s == "charging" || s == "full" || s == "not charging" {
                    return true;
                }
                if s == "discharging" {
                    return false;
                }
            }
        }
    }
    !found_battery
}

fn parse_args() -> MinerConfig {
    let args: Vec<String> = std::env::args().collect();
    let default_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);

    let mut cfg = MinerConfig {
        rpc_url: "127.0.0.1:19332".to_string(),
        coinbase_address: None,
        threads: default_threads,
        cpu_limit_pct: 100,
        max_temp_c: 80.0,
        min_battery_pct: 25,
        pause_file: None,
        benchmark_secs: None,
        max_blocks: None,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--rpc" if i + 1 < args.len() => {
                cfg.rpc_url = args[i + 1]
                    .trim_start_matches("http://")
                    .trim_end_matches('/')
                    .to_string();
                i += 2;
            }
            "--address" if i + 1 < args.len() => {
                cfg.coinbase_address = Some(args[i + 1].clone());
                i += 2;
            }
            "--threads" if i + 1 < args.len() => {
                cfg.threads = args[i + 1].parse().unwrap_or(default_threads).max(1);
                i += 2;
            }
            "--cpu-limit" if i + 1 < args.len() => {
                cfg.cpu_limit_pct = args[i + 1].parse::<u8>().unwrap_or(100).clamp(5, 100);
                i += 2;
            }
            "--max-temp-c" if i + 1 < args.len() => {
                cfg.max_temp_c = args[i + 1].parse().unwrap_or(80.0);
                i += 2;
            }
            "--min-battery-pct" if i + 1 < args.len() => {
                cfg.min_battery_pct = args[i + 1].parse::<u8>().unwrap_or(25).min(100);
                i += 2;
            }
            "--pause-file" if i + 1 < args.len() => {
                cfg.pause_file = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--blocks" if i + 1 < args.len() => {
                cfg.max_blocks = Some(args[i + 1].parse::<u64>().unwrap_or(1).max(1));
                i += 2;
            }
            "--once" => {
                cfg.max_blocks = Some(1);
                i += 1;
            }
            "--benchmark" => {
                let secs = if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    i += 2;
                    args[i - 1].parse().unwrap_or(2)
                } else {
                    i += 1;
                    2
                };
                cfg.benchmark_secs = Some(secs.max(1));
            }
            "--help" | "-h" => {
                println!(
                    "Sardhan CPU/ARM64 Miner (sdr-miner)
Usage: sdr-miner [OPTIONS]
  --rpc <host:port>         Node JSON-RPC endpoint (default: 127.0.0.1:19332)
  --address <tsdr1...>      Reward address (Bech32m)
  --threads <n>             Worker threads (default: {default_threads})
  --cpu-limit <5..100>      Max CPU duty cycle percent (default: 100)
  --max-temp-c <deg>        Pause threshold in Celsius (default: 80.0)
  --min-battery-pct <pct>   Pause when discharging below pct (default: 25)
  --pause-file <path>       Pause while this file exists; remove to resume
  --blocks <n>              Stop cleanly after mining n accepted blocks
  --once                    Shortcut for --blocks 1
  --benchmark [seconds]     Benchmark SardhanHash vs Verus-style & RandomX-style CPU loops"
                );
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown argument: {other}");
                std::process::exit(1);
            }
        }
    }
    cfg
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

/// Reference Verus-style CPU pipeline simulation (integer carryless + AES/ARX-like lane mixing, stateless)
fn bench_verus_style_mix(input: &[u8; 64]) -> [u8; 32] {
    let mut acc = [0u64; 8];
    for i in 0..8 {
        acc[i] = u64::from_le_bytes(input[i * 8..i * 8 + 8].try_into().unwrap());
    }
    for round in 0..32u64 {
        for lane in 0..8 {
            let next = (lane + 1) & 7;
            let carry_mix = acc[lane].wrapping_mul(0x9e37_79b9_7f4a_7c15 ^ round);
            acc[lane] = acc[lane].rotate_left(17) ^ carry_mix.wrapping_add(acc[next]);
        }
    }
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..i * 8 + 8].copy_from_slice(&(acc[i] ^ acc[i + 4]).to_le_bytes());
    }
    hash_blake3(&out)
}

/// Reference RandomX-style 2 MiB random memory VM execution loop (pure-Rust scalar baseline)
fn bench_randomx_style_vm(seed: &[u8; 32], scratchpad: &mut [u64]) -> [u8; 32] {
    let len = scratchpad.len();
    let mut r = [0u64; 4];
    for i in 0..4 {
        r[i] = u64::from_le_bytes(seed[i * 8..i * 8 + 8].try_into().unwrap());
    }
    for step in 0..2048u64 {
        let idx = ((r[0] ^ step) as usize) % len;
        let mem = scratchpad[idx];
        r[1] = r[1].wrapping_add(mem).rotate_left(19);
        r[2] ^= r[1].wrapping_mul(0xbf58_476d_1ce4_e5b9);
        r[3] = r[3].wrapping_sub(r[2] ^ step);
        r[0] = r[0].rotate_right(11) ^ r[3];
        scratchpad[idx] = r[0] ^ r[2];
    }
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..i * 8 + 8].copy_from_slice(&r[i].to_le_bytes());
    }
    hash_blake3(&out)
}

fn run_benchmarks(secs: u64, threads: usize) {
    println!("========================================================================");
    println!("       SARDHAN HASH EMPIRICAL CPU BENCHMARK REPORT (HONEST MODE)        ");
    println!("========================================================================");
    println!("Host Architecture: {}", std::env::consts::ARCH);
    println!("Host OS:           {}", std::env::consts::OS);
    println!("Threads tested:    1 (single-core) and {} (multi-core)", threads);
    println!("Duration/test:     {}s per workload", secs);
    println!("------------------------------------------------------------------------");

    // 1. SardhanHash Single-Thread
    let mut hdr = [0x53u8; 196];
    let dur = Duration::from_secs(secs);
    let t0 = Instant::now();
    let mut count_st = 0u64;
    while t0.elapsed() < dur {
        hdr[121..129].copy_from_slice(&count_st.to_le_bytes());
        let _ = std::hint::black_box(sardhan_hash(&hdr));
        count_st += 1;
    }
    let st_secs = t0.elapsed().as_secs_f64();
    let sardhan_st_hps = (count_st as f64) / st_secs;

    // 2. SardhanHash Multi-Thread
    let total_mt = Arc::new(AtomicU64::new(0));
    let t_mt = Instant::now();
    let mut handles = Vec::new();
    for tid in 0..threads {
        let tot = Arc::clone(&total_mt);
        handles.push(std::thread::spawn(move || {
            let mut local_hdr = [0x53u8; 196];
            let mut n = (tid as u64) << 40;
            let start = Instant::now();
            let mut local_count = 0u64;
            while start.elapsed() < dur {
                local_hdr[121..129].copy_from_slice(&n.to_le_bytes());
                let _ = std::hint::black_box(sardhan_hash(&local_hdr));
                n += 1;
                local_count += 1;
            }
            tot.fetch_add(local_count, Ordering::Relaxed);
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let mt_secs = t_mt.elapsed().as_secs_f64();
    let count_mt = total_mt.load(Ordering::Relaxed);
    let sardhan_mt_hps = (count_mt as f64) / mt_secs;

    // 3. BLAKE3-256 single-thread baseline
    let t_b3 = Instant::now();
    let mut count_b3 = 0u64;
    while t_b3.elapsed() < dur {
        hdr[121..129].copy_from_slice(&count_b3.to_le_bytes());
        let _ = std::hint::black_box(hash_blake3(&hdr));
        count_b3 += 1;
    }
    let b3_hps = (count_b3 as f64) / t_b3.elapsed().as_secs_f64();

    // 4. Verus-style CPU mix simulation (single-thread)
    let mut v_in = [0x42u8; 64];
    let t_vr = Instant::now();
    let mut count_vr = 0u64;
    while t_vr.elapsed() < dur {
        v_in[0..8].copy_from_slice(&count_vr.to_le_bytes());
        let _ = std::hint::black_box(bench_verus_style_mix(&v_in));
        count_vr += 1;
    }
    let vr_hps = (count_vr as f64) / t_vr.elapsed().as_secs_f64();

    // 5. RandomX-style 2 MiB VM loop simulation (single-thread)
    let mut rx_pad = vec![0x5a5a_5a5a_5a5a_5a5au64; (2 * 1024 * 1024) / 8];
    let mut rx_seed = [0x11u8; 32];
    let t_rx = Instant::now();
    let mut count_rx = 0u64;
    while t_rx.elapsed() < dur {
        rx_seed[0..8].copy_from_slice(&count_rx.to_le_bytes());
        let _ = std::hint::black_box(bench_randomx_style_vm(&rx_seed, &mut rx_pad));
        count_rx += 1;
    }
    let rx_hps = (count_rx as f64) / t_rx.elapsed().as_secs_f64();

    println!(
        "{:<34} {:>12} {:>14} {:>12}",
        "Algorithm / Workload", "Throughput", "Avg Latency", "Scratchpad"
    );
    println!("------------------------------------------------------------------------");
    println!(
        "{:<34} {:>9.2} H/s {:>11.3} ms {:>12}",
        "SardhanHash (1T actual)",
        sardhan_st_hps,
        1000.0 / sardhan_st_hps.max(0.001),
        "8 MiB"
    );
    println!(
        "{:<34} {:>9.2} H/s {:>11.3} ms {:>12}",
        format!("SardhanHash ({}T actual)", threads),
        sardhan_mt_hps,
        1000.0 / sardhan_mt_hps.max(0.001),
        "8 MiB/thr"
    );
    println!(
        "{:<34} {:>9.2} H/s {:>11.4} ms {:>12}",
        "RandomX-style 2MiB VM sim (1T)",
        rx_hps,
        1000.0 / rx_hps.max(0.001),
        "2 MiB"
    );
    println!(
        "{:<34} {:>9.2} H/s {:>11.5} ms {:>12}",
        "Verus-style ARX mix sim (1T)",
        vr_hps,
        1000.0 / vr_hps.max(0.001),
        "Stateless"
    );
    println!(
        "{:<34} {:>9.2} H/s {:>11.6} ms {:>12}",
        "BLAKE3-256 Header Hash (1T)",
        b3_hps,
        1000.0 / b3_hps.max(0.001),
        "0 B"
    );
    println!("========================================================================");
    println!("HONESTY & GPU-RESISTANCE DISCLAIMER:");
    println!("- Numbers above are real measured timings on this host ({}).", std::env::consts::ARCH);
    println!("- RandomX-style & Verus-style entries are pure-Rust reference simulations,");
    println!("  not external JIT/AES-NI reference binaries.");
    println!("- GPU / ASIC resistance of SardhanHash CANNOT be claimed until tested on");
    println!("  physical CUDA/OpenCL/Metal GPU hardware comparing energy/hash vs CPU.");
    println!("- Notably, SardhanHash spends most of its time in BLAKE3 XOF scratchpad");
    println!("  fill (131,072 x 64B chunks) with only 64 branching reads afterward;");
    println!("  see Audit Notes on optimizing scratchpad-to-mixing ratio before Mainnet.");
    println!("========================================================================");
}

fn main() {
    let cfg = parse_args();
    println!("=== SARDHAN CHAIN CPU/ARM64 MINER (`sdr-miner` v0.2.0) ===");
    println!(
        "Arch: {} | Threads: {} | CPU Limit: {}% | Max Temp: {:.1}°C | Min Bat: {}%",
        std::env::consts::ARCH,
        cfg.threads,
        cfg.cpu_limit_pct,
        cfg.max_temp_c,
        cfg.min_battery_pct
    );

    if let Some(secs) = cfg.benchmark_secs {
        run_benchmarks(secs, cfg.threads);
        return;
    }

    let paused = Arc::new(AtomicBool::new(false));
    let running = Arc::new(AtomicBool::new(true));

    // Spawn interactive pause/resume stdin listener (p = pause, r = resume, q = quit)
    let p_stdin = Arc::clone(&paused);
    let r_stdin = Arc::clone(&running);
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            match line.trim().to_ascii_lowercase().as_str() {
                "p" | "pause" => {
                    p_stdin.store(true, Ordering::SeqCst);
                    println!("[MINER] Paused by operator command.");
                }
                "r" | "resume" => {
                    p_stdin.store(false, Ordering::SeqCst);
                    println!("[MINER] Resumed by operator command.");
                }
                "q" | "quit" | "exit" => {
                    r_stdin.store(false, Ordering::SeqCst);
                    break;
                }
                _ => {}
            }
        }
    });

    let total_hashes = Arc::new(AtomicU64::new(0));
    let start_all = Instant::now();
    let mut accepted_blocks = 0u64;

    while running.load(Ordering::Relaxed) {
        if let Some(limit) = cfg.max_blocks {
            if accepted_blocks >= limit {
                break;
            }
        }
        // 1. Check pause file
        if let Some(ref pf) = cfg.pause_file {
            if pf.exists() {
                println!("[MINER] Paused via pause-file {:?}. Waiting...", pf);
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        }
        if paused.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        // 2. Check thermal and battery protection
        let health = HardwareHealth::sample();
        if let Some(reason) = health.should_throttle(cfg.max_temp_c, cfg.min_battery_pct) {
            println!("[PROTECTION] {reason} — pausing 5s");
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }

        // 3. Request block template from sdrd JSON-RPC
        let params = if let Some(ref addr) = cfg.coinbase_address {
            json!([addr])
        } else {
            json!([])
        };
        let tmpl = match rpc_call(&cfg.rpc_url, "getblocktemplate", params) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[MINER] Waiting for RPC template ({e})...");
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        let block_hex = tmpl["block_hex"].as_str().unwrap_or("");
        let Ok(block_bytes) = hex::decode(block_hex) else {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        };
        let Ok(template_block) = Block::from_bytes(&block_bytes) else {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        };

        let height = template_block.header.block_height;
        let target = compact_to_target(template_block.header.consensus_bits);
        let found = Arc::new(AtomicBool::new(false));
        let stop_round = Arc::new(AtomicBool::new(false));
        let cpu_limit = cfg.cpu_limit_pct;

        let mut handles = Vec::with_capacity(cfg.threads);
        for tid in 0..cfg.threads {
            let mut blk = template_block.clone();
            let f_flag = Arc::clone(&found);
            let s_flag = Arc::clone(&stop_round);
            let h_counter = Arc::clone(&total_hashes);
            let p_flag = Arc::clone(&paused);
            let r_flag = Arc::clone(&running);

            handles.push(std::thread::spawn(move || -> Option<Block> {
                let mut nonce = (tid as u64) + 1;
                let mut batch_start = Instant::now();
                for iter in 0..25_000u64 {
                    if f_flag.load(Ordering::Relaxed)
                        || s_flag.load(Ordering::Relaxed)
                        || p_flag.load(Ordering::Relaxed)
                        || !r_flag.load(Ordering::Relaxed)
                    {
                        return None;
                    }
                    blk.header.pow_nonce = nonce;
                    let hash = sardhan_hash(&blk.header.to_bytes());
                    h_counter.fetch_add(1, Ordering::Relaxed);
                    if hash_less_than_target(&hash, &target) {
                        f_flag.store(true, Ordering::SeqCst);
                        return Some(blk);
                    }
                    nonce += cfg.threads as u64;

                    // Duty-cycle CPU throttling when cpu_limit < 100%
                    if cpu_limit < 100 && (iter % 16 == 15) {
                        let work_ms = batch_start.elapsed().as_millis() as u64;
                        let sleep_ms = work_ms * (100 - cpu_limit as u64) / (cpu_limit as u64);
                        if sleep_ms > 0 {
                            std::thread::sleep(Duration::from_millis(sleep_ms.min(500)));
                        }
                        batch_start = Instant::now();
                    }
                }
                None
            }));
        }

        let mut winning_block = None;
        for h in handles {
            if let Ok(Some(b)) = h.join() {
                if winning_block.is_none() {
                    winning_block = Some(b);
                }
            }
        }

        let elapsed = start_all.elapsed().as_secs_f64().max(0.001);
        let hashes_done = total_hashes.load(Ordering::Relaxed);
        let hps = (hashes_done as f64) / elapsed;

        if let Some(winner) = winning_block {
            let bhex = hex::encode(winner.to_bytes());
            match rpc_call(&cfg.rpc_url, "submitblock", json!([bhex])) {
                Ok(res) => {
                    accepted_blocks += 1;
                    println!(
                        "[BLOCK ACCEPTED] Height #{} | Hash: 0x{}... | Avg Hashrate: {:.2} H/s",
                        height,
                        &hex::encode(winner.block_hash())[..16],
                        hps
                    );
                    let _ = res;
                }
                Err(e) => eprintln!("[SUBMIT FAILED] {e}"),
            }
        } else {
            println!(
                "[MINING] Height #{} | Total Hashes: {} | Hashrate: {:.2} H/s | Temp: {:?}°C",
                height, hashes_done, hps, health.max_temp_c
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_and_battery_protection_gates() {
        let hot = HardwareHealth {
            max_temp_c: Some(85.0),
            battery_pct: Some(90),
            on_ac_power: true,
        };
        assert!(hot.should_throttle(80.0, 25).is_some());

        let low_bat = HardwareHealth {
            max_temp_c: Some(55.0),
            battery_pct: Some(15),
            on_ac_power: false,
        };
        assert!(low_bat.should_throttle(80.0, 25).is_some());

        let nominal = HardwareHealth {
            max_temp_c: Some(55.0),
            battery_pct: Some(15),
            on_ac_power: true, // Plugged into AC charger
        };
        assert!(nominal.should_throttle(80.0, 25).is_none());
    }
}
