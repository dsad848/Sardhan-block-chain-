//! # Sardhan Peer-to-Peer Protocol & Decentralized Mesh Engine
//!
//! Framing:
//! [Magic: 4B] [Command: 12B ASCII null-padded] [PayloadLength: 4B] [Checksum: 4B] [Payload]
//!
//! Features:
//! - Multi-peer asynchronous Tokio TCP mesh with persistent `peers.json`
//! - /16 Subnet Eclipse attack mitigation (max 2 peers per public /16 subnet)
//! - Peer scoring (0..100) and misbehavior auto-banning
//! - Complete protocol: version, verack, getaddr, addr, ping, pong, getheaders, headers, inv, getdata, block, tx
//! - Automatic header/block catch-up sync & network partition healing

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use rand::RngCore;
use sdr_crypto::hash_blake3;
use sdr_ledger::{current_unix_timestamp, Ledger, LedgerError};
use sdr_mempool::{Mempool, MempoolError};
use sdr_primitives::{
    Block, BlockHeader, Transaction, BLOCK_HEADER_SIZE, SARDHAN_MAINNET_MAGIC,
    SARDHAN_REGTEST_MAGIC, SARDHAN_TESTNET_MAGIC,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout, Duration};

pub const DEFAULT_P2P_PORT_MAINNET: u16 = 9333;
pub const DEFAULT_P2P_PORT_TESTNET: u16 = 19333;
pub const MAX_PEERS_PER_SUBNET_16: usize = 2;
pub const MAX_CONNECTED_PEERS: usize = 64;
pub const BAN_DURATION_SECS: u64 = 3600;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum P2pError {
    #[error("Invalid network magic byte: {0:#x}")]
    InvalidMagic(u32),
    #[error("Message checksum mismatch")]
    ChecksumMismatch,
    #[error("Command string is malformed")]
    InvalidCommand,
    #[error("Payload exceeds maximum size")]
    PayloadTooLarge,
    #[error("Peer subnet /16 eclipse limit reached")]
    SubnetLimitReached,
    #[error("Peer banned until {0}")]
    PeerBanned(u64),
    #[error("IO error: {0}")]
    Io(String),
}

/// A framed wire protocol message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireMessage {
    pub magic: u32,
    pub command: String,
    pub payload: Vec<u8>,
}

impl WireMessage {
    pub fn new(magic: u32, command: &str, payload: Vec<u8>) -> Self {
        Self {
            magic,
            command: command.to_string(),
            payload,
        }
    }

    /// Serialize framed message to binary wire format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(24 + self.payload.len());
        buf.write_u32::<LittleEndian>(self.magic).unwrap();

        let mut cmd_bytes = [0u8; 12];
        let bytes = self.command.as_bytes();
        let copy_len = bytes.len().min(12);
        cmd_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
        buf.write_all(&cmd_bytes).unwrap();

        buf.write_u32::<LittleEndian>(self.payload.len() as u32)
            .unwrap();

        let hash = hash_blake3(&self.payload);
        buf.write_all(&hash[0..4]).unwrap();
        buf.write_all(&self.payload).unwrap();
        buf
    }

    /// Deserialize message from binary wire format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, P2pError> {
        if bytes.len() < 24 {
            return Err(P2pError::Io("Buffer too small for wire header".into()));
        }

        let mut r = Cursor::new(bytes);
        let magic = r
            .read_u32::<LittleEndian>()
            .map_err(|e| P2pError::Io(e.to_string()))?;

        if magic != SARDHAN_MAINNET_MAGIC
            && magic != SARDHAN_TESTNET_MAGIC
            && magic != SARDHAN_REGTEST_MAGIC
        {
            return Err(P2pError::InvalidMagic(magic));
        }

        let mut cmd_buf = [0u8; 12];
        r.read_exact(&mut cmd_buf)
            .map_err(|e| P2pError::Io(e.to_string()))?;
        let command = String::from_utf8_lossy(&cmd_buf)
            .trim_matches('\0')
            .to_string();

        let payload_len = r
            .read_u32::<LittleEndian>()
            .map_err(|e| P2pError::Io(e.to_string()))? as usize;
        if payload_len > 8 * 1024 * 1024 {
            return Err(P2pError::PayloadTooLarge);
        }

        let mut checksum = [0u8; 4];
        r.read_exact(&mut checksum)
            .map_err(|e| P2pError::Io(e.to_string()))?;

        if bytes.len() < 24 + payload_len {
            return Err(P2pError::Io("Incomplete payload".into()));
        }

        let payload = bytes[24..24 + payload_len].to_vec();
        let expected_hash = hash_blake3(&payload);
        if checksum != expected_hash[0..4] {
            return Err(P2pError::ChecksumMismatch);
        }

        Ok(Self {
            magic,
            command,
            payload,
        })
    }
}

/// Node network version payload for initial TCP handshake
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionPayload {
    pub protocol_version: u32,
    pub services: u64,
    pub timestamp: u64,
    pub best_height: u64,
    #[serde(default)]
    pub best_hash_hex: String,
    #[serde(default)]
    pub listen_port: u16,
    #[serde(default)]
    pub node_nonce: u64,
    pub user_agent: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum InvKind {
    Tx = 1,
    Block = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvVector {
    pub kind: InvKind,
    pub hash_hex: String,
}

/// Persisted peer database entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRecord {
    pub addr: String,
    pub score: i32,
    pub last_seen: u64,
    pub banned_until: u64,
}

/// Live connected peer snapshot for RPC `getpeerinfo` & monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSnapshot {
    pub addr: String,
    pub listen_addr: Option<String>,
    pub inbound: bool,
    pub best_height: u64,
    pub score: i32,
    pub user_agent: String,
    pub latency_ms: u64,
    pub connected_since: u64,
}

/// Subnet bucket extraction for /16 eclipse resistance (returns None for loopback so local multi-node works)
pub fn public_subnet_16(ip: IpAddr) -> Option<[u8; 2]> {
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_loopback() {
                None
            } else {
                let octets = v4.octets();
                Some([octets[0], octets[1]])
            }
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() {
                None
            } else {
                let segs = v6.segments();
                Some([(segs[0] >> 8) as u8, (segs[0] & 0xff) as u8])
            }
        }
    }
}

/// Stateful per-subnet connection tracker preventing eclipse attacks
#[derive(Debug, Default)]
pub struct EclipseGuard {
    subnet_counts: HashMap<[u8; 2], usize>,
}

impl EclipseGuard {
    pub fn can_accept(&self, ip: IpAddr) -> bool {
        if let Some(sub) = public_subnet_16(ip) {
            self.subnet_counts.get(&sub).copied().unwrap_or(0) < MAX_PEERS_PER_SUBNET_16
        } else {
            true
        }
    }

    pub fn register(&mut self, ip: IpAddr) {
        if let Some(sub) = public_subnet_16(ip) {
            *self.subnet_counts.entry(sub).or_insert(0) += 1;
        }
    }

    pub fn unregister(&mut self, ip: IpAddr) {
        if let Some(sub) = public_subnet_16(ip) {
            if let Some(c) = self.subnet_counts.get_mut(&sub) {
                *c = c.saturating_sub(1);
                if *c == 0 {
                    self.subnet_counts.remove(&sub);
                }
            }
        }
    }
}

struct ConnectedPeer {
    addr: SocketAddr,
    listen_addr: Option<String>,
    inbound: bool,
    best_height: u64,
    score: i32,
    user_agent: String,
    latency_ms: u64,
    connected_since: u64,
    node_nonce: u64,
    tx_sender: mpsc::Sender<WireMessage>,
    shutdown_tx: mpsc::Sender<()>,
}

/// Asynchronous P2P Peer Network Manager (also retains compatibility helpers)
pub struct P2pNetwork {
    pub magic: u32,
    pub listen_port: u16,
    pub connected_peers: Arc<RwLock<Vec<SocketAddr>>>,
}

impl P2pNetwork {
    pub fn new(magic: u32, listen_port: u16) -> Self {
        Self {
            magic,
            listen_port,
            connected_peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Read a framed WireMessage from an async TCP stream
    pub async fn read_message(stream: &mut TcpStream) -> Result<WireMessage, P2pError> {
        use tokio::io::AsyncReadExt;
        let mut header_buf = [0u8; 24];
        stream
            .read_exact(&mut header_buf)
            .await
            .map_err(|e| P2pError::Io(e.to_string()))?;

        let magic = u32::from_le_bytes(header_buf[0..4].try_into().unwrap());
        if magic != SARDHAN_MAINNET_MAGIC
            && magic != SARDHAN_TESTNET_MAGIC
            && magic != SARDHAN_REGTEST_MAGIC
        {
            return Err(P2pError::InvalidMagic(magic));
        }

        let command = String::from_utf8_lossy(&header_buf[4..16])
            .trim_matches('\0')
            .to_string();
        let payload_len = u32::from_le_bytes(header_buf[16..20].try_into().unwrap()) as usize;
        let mut checksum = [0u8; 4];
        checksum.copy_from_slice(&header_buf[20..24]);

        if payload_len > 8 * 1024 * 1024 {
            return Err(P2pError::PayloadTooLarge);
        }

        let mut payload = vec![0u8; payload_len];
        stream
            .read_exact(&mut payload)
            .await
            .map_err(|e| P2pError::Io(e.to_string()))?;

        let expected_hash = hash_blake3(&payload);
        if checksum != expected_hash[0..4] {
            return Err(P2pError::ChecksumMismatch);
        }

        Ok(WireMessage {
            magic,
            command,
            payload,
        })
    }

    /// Write a framed WireMessage to an async TCP stream
    pub async fn write_message(stream: &mut TcpStream, msg: &WireMessage) -> Result<(), P2pError> {
        use tokio::io::AsyncWriteExt;
        let bytes = msg.to_bytes();
        stream
            .write_all(&bytes)
            .await
            .map_err(|e| P2pError::Io(e.to_string()))?;
        stream
            .flush()
            .await
            .map_err(|e| P2pError::Io(e.to_string()))?;
        Ok(())
    }

    /// Perform mutual P2P handshake (`version` -> `verack`)
    pub async fn perform_handshake(
        stream: &mut TcpStream,
        magic: u32,
        our_height: u64,
    ) -> Result<u64, P2pError> {
        let version_info = VersionPayload {
            protocol_version: 2,
            services: 1,
            timestamp: current_unix_timestamp(),
            best_height: our_height,
            best_hash_hex: String::new(),
            listen_port: 0,
            node_nonce: rand::rngs::OsRng.next_u64(),
            user_agent: "/SardhanCore:0.2.0/".to_string(),
        };
        let payload = serde_json::to_vec(&version_info).unwrap();
        let version_msg = WireMessage::new(magic, "version", payload);
        Self::write_message(stream, &version_msg).await?;

        let peer_version_msg = Self::read_message(stream).await?;
        if peer_version_msg.command != "version" {
            return Err(P2pError::InvalidCommand);
        }
        let peer_info: VersionPayload = serde_json::from_slice(&peer_version_msg.payload)
            .map_err(|e| P2pError::Io(e.to_string()))?;
        if peer_version_msg.magic != magic {
            return Err(P2pError::InvalidMagic(peer_version_msg.magic));
        }

        let verack_msg = WireMessage::new(magic, "verack", vec![]);
        Self::write_message(stream, &verack_msg).await?;

        let peer_verack = Self::read_message(stream).await?;
        if peer_verack.command != "verack" {
            return Err(P2pError::InvalidCommand);
        }

        Ok(peer_info.best_height)
    }
}

/// Complete Decentralized Multi-Peer P2P Coordinator
pub struct P2pManager {
    pub magic: u32,
    pub listen_addr: SocketAddr,
    pub node_nonce: u64,
    pub ledger: Arc<RwLock<Ledger>>,
    pub mempool: Arc<RwLock<Mempool>>,
    pub datadir: Option<PathBuf>,
    peers: RwLock<HashMap<SocketAddr, ConnectedPeer>>,
    known_peers: RwLock<HashMap<String, PeerRecord>>,
    eclipse_guard: RwLock<EclipseGuard>,
    pending_pings: RwLock<HashMap<(SocketAddr, u64), u64>>,
    banned_ips: RwLock<HashMap<IpAddr, u64>>,
}

impl P2pManager {
    pub fn new(
        magic: u32,
        listen_addr: SocketAddr,
        ledger: Arc<RwLock<Ledger>>,
        mempool: Arc<RwLock<Mempool>>,
        datadir: Option<PathBuf>,
    ) -> Arc<Self> {
        let mut initial_known = HashMap::new();
        if let Some(ref dir) = datadir {
            let peers_path = dir.join("peers.json");
            if let Ok(raw) = std::fs::read_to_string(peers_path) {
                if let Ok(list) = serde_json::from_str::<Vec<PeerRecord>>(&raw) {
                    for rec in list {
                        initial_known.insert(rec.addr.clone(), rec);
                    }
                }
            }
        }

        Arc::new(Self {
            magic,
            listen_addr,
            node_nonce: rand::rngs::OsRng.next_u64(),
            ledger,
            mempool,
            datadir,
            peers: RwLock::new(HashMap::new()),
            known_peers: RwLock::new(initial_known),
            eclipse_guard: RwLock::new(EclipseGuard::default()),
            pending_pings: RwLock::new(HashMap::new()),
            banned_ips: RwLock::new(HashMap::new()),
        })
    }

    pub async fn persist_peers(&self) {
        let Some(ref dir) = self.datadir else {
            return;
        };
        let list: Vec<PeerRecord> = self.known_peers.read().await.values().cloned().collect();
        if let Ok(json) = serde_json::to_string_pretty(&list) {
            let tmp = dir.join("peers.json.tmp");
            let dst = dir.join("peers.json");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &dst);
            }
        }
    }

    /// Start inbound TCP listener and background sync/keepalive task
    pub async fn start(self: &Arc<Self>) -> Result<SocketAddr, P2pError> {
        let listener = TcpListener::bind(self.listen_addr)
            .await
            .map_err(|e| P2pError::Io(e.to_string()))?;
        let bound_addr = listener.local_addr().map_err(|e| P2pError::Io(e.to_string()))?;

        let this = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let mgr = Arc::clone(&this);
                        tokio::spawn(async move {
                            let _ = mgr.handle_connection(stream, addr, true).await;
                        });
                    }
                    Err(e) => {
                        log::warn!("P2P accept error: {e}");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });

        // Periodic sync + ping + peer discovery loop
        let sync_mgr = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(2));
            loop {
                ticker.tick().await;
                sync_mgr.sync_with_peers().await;
            }
        });

        Ok(bound_addr)
    }

    /// Connect outbound to a peer address string ("127.0.0.1:19333")
    pub async fn connect_peer(self: &Arc<Self>, target: &str) -> Result<(), P2pError> {
        let addr: SocketAddr = target
            .parse()
            .map_err(|e| P2pError::Io(format!("Invalid socket addr '{target}': {e}")))?;

        if let Some(&until) = self.banned_ips.read().await.get(&addr.ip()) {
            if current_unix_timestamp() < until {
                return Err(P2pError::PeerBanned(until));
            }
        }

        {
            let peers = self.peers.read().await;
            if peers.contains_key(&addr)
                || peers
                    .values()
                    .any(|p| p.listen_addr.as_deref() == Some(target))
            {
                return Ok(());
            }
            if peers.len() >= MAX_CONNECTED_PEERS {
                return Ok(());
            }
        }

        if !self.eclipse_guard.read().await.can_accept(addr.ip()) {
            return Err(P2pError::SubnetLimitReached);
        }

        let stream = timeout(Duration::from_secs(3), TcpStream::connect(addr))
            .await
            .map_err(|_| P2pError::Io("Connection timeout".into()))?
            .map_err(|e| P2pError::Io(e.to_string()))?;

        // Record in persistent peer DB
        {
            let mut known = self.known_peers.write().await;
            known.entry(target.to_string()).or_insert(PeerRecord {
                addr: target.to_string(),
                score: 100,
                last_seen: current_unix_timestamp(),
                banned_until: 0,
            });
        }
        self.persist_peers().await;

        let mgr = Arc::clone(self);
        tokio::spawn(async move {
            let _ = mgr.handle_connection(stream, addr, false).await;
        });

        Ok(())
    }

    /// Disconnect a peer (by socket addr or listen_addr) — useful for partition testing
    pub async fn disconnect_peer(&self, addr_or_listen: &str) -> bool {
        let mut removed = false;
        let targets: Vec<(SocketAddr, mpsc::Sender<()>)> = {
            let peers = self.peers.read().await;
            peers
                .iter()
                .filter(|(sa, p)| {
                    sa.to_string() == addr_or_listen
                        || p.listen_addr.as_deref() == Some(addr_or_listen)
                })
                .map(|(sa, p)| (*sa, p.shutdown_tx.clone()))
                .collect()
        };
        for (sa, stop) in targets {
            let _ = stop.send(()).await;
            self.peers.write().await.remove(&sa);
            self.eclipse_guard.write().await.unregister(sa.ip());
            removed = true;
        }
        removed
    }

    /// Disconnect all peers (simulate complete network partition)
    pub async fn disconnect_all(&self) -> usize {
        let all: Vec<(SocketAddr, mpsc::Sender<()>)> = {
            let peers = self.peers.read().await;
            peers
                .iter()
                .map(|(sa, p)| (*sa, p.shutdown_tx.clone()))
                .collect()
        };
        let count = all.len();
        for (sa, stop) in all {
            let _ = stop.send(()).await;
            self.peers.write().await.remove(&sa);
            self.eclipse_guard.write().await.unregister(sa.ip());
        }
        count
    }

    async fn handle_connection(
        self: &Arc<Self>,
        mut stream: TcpStream,
        peer_addr: SocketAddr,
        inbound: bool,
    ) -> Result<(), P2pError> {
        let now = current_unix_timestamp();
        if let Some(&until) = self.banned_ips.read().await.get(&peer_addr.ip()) {
            if now < until {
                return Err(P2pError::PeerBanned(until));
            }
        }
        if !self.eclipse_guard.read().await.can_accept(peer_addr.ip()) {
            return Err(P2pError::SubnetLimitReached);
        }

        let (our_height, our_hash) = {
            let l = self.ledger.read().await;
            (l.best_height, hex::encode(l.best_block_hash))
        };

        let version_info = VersionPayload {
            protocol_version: 2,
            services: 1,
            timestamp: now,
            best_height: our_height,
            best_hash_hex: our_hash,
            listen_port: self.listen_addr.port(),
            node_nonce: self.node_nonce,
            user_agent: "/SardhanCore:0.2.0/".to_string(),
        };
        let version_msg =
            WireMessage::new(self.magic, "version", serde_json::to_vec(&version_info).unwrap());
        P2pNetwork::write_message(&mut stream, &version_msg).await?;

        let peer_version_msg = timeout(Duration::from_secs(4), P2pNetwork::read_message(&mut stream))
            .await
            .map_err(|_| P2pError::Io("Handshake timeout".into()))??;
        if peer_version_msg.command != "version" || peer_version_msg.magic != self.magic {
            return Err(P2pError::InvalidCommand);
        }
        let peer_info: VersionPayload = serde_json::from_slice(&peer_version_msg.payload)
            .map_err(|e| P2pError::Io(e.to_string()))?;

        // Reject self-connection or duplicate node_nonce
        if peer_info.node_nonce == self.node_nonce {
            return Ok(());
        }
        {
            let peers = self.peers.read().await;
            if peers.values().any(|p| p.node_nonce == peer_info.node_nonce) {
                return Ok(());
            }
        }

        P2pNetwork::write_message(&mut stream, &WireMessage::new(self.magic, "verack", vec![]))
            .await?;
        let peer_verack = timeout(Duration::from_secs(4), P2pNetwork::read_message(&mut stream))
            .await
            .map_err(|_| P2pError::Io("Verack timeout".into()))??;
        if peer_verack.command != "verack" {
            return Err(P2pError::InvalidCommand);
        }

        let listen_addr = if peer_info.listen_port > 0 {
            Some(format!("{}:{}", peer_addr.ip(), peer_info.listen_port))
        } else {
            None
        };

        let (tx_sender, mut tx_rx) = mpsc::channel::<WireMessage>(128);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(4);

        self.eclipse_guard.write().await.register(peer_addr.ip());
        self.peers.write().await.insert(
            peer_addr,
            ConnectedPeer {
                addr: peer_addr,
                listen_addr: listen_addr.clone(),
                inbound,
                best_height: peer_info.best_height,
                score: 100,
                user_agent: peer_info.user_agent,
                latency_ms: 0,
                connected_since: now,
                node_nonce: peer_info.node_nonce,
                tx_sender: tx_sender.clone(),
                shutdown_tx,
            },
        );

        if let Some(ref la) = listen_addr {
            self.known_peers.write().await.insert(
                la.clone(),
                PeerRecord {
                    addr: la.clone(),
                    score: 100,
                    last_seen: now,
                    banned_until: 0,
                },
            );
            self.persist_peers().await;
        }

        // Request addresses & headers immediately on connect
        let _ = tx_sender
            .send(WireMessage::new(self.magic, "getaddr", vec![]))
            .await;
        if peer_info.best_height > our_height {
            let locator = self.build_locator().await;
            let _ = tx_sender
                .send(WireMessage::new(
                    self.magic,
                    "getheaders",
                    serde_json::to_vec(&locator).unwrap(),
                ))
                .await;
        }

        let (mut read_half, mut write_half) = stream.into_split();

        let writer_handle = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            loop {
                tokio::select! {
                    Some(msg) = tx_rx.recv() => {
                        let bytes = msg.to_bytes();
                        if write_half.write_all(&bytes).await.is_err() {
                            break;
                        }
                        let _ = write_half.flush().await;
                    }
                    _ = shutdown_rx.recv() => break,
                }
            }
        });

        let expected_magic = self.magic;
        loop {
            let msg = match read_message_half(&mut read_half, expected_magic).await {
                Ok(m) => m,
                Err(P2pError::ChecksumMismatch) | Err(P2pError::InvalidMagic(_)) => {
                    self.penalize_peer(peer_addr, 50).await;
                    break;
                }
                Err(_) => break,
            };

            self.dispatch_message(peer_addr, &tx_sender, msg).await;

            // Disconnect immediately if peer dropped below ban threshold
            let should_drop = self
                .peers
                .read()
                .await
                .get(&peer_addr)
                .map(|p| p.score <= 0)
                .unwrap_or(true);
            if should_drop {
                break;
            }
        }

        writer_handle.abort();
        self.peers.write().await.remove(&peer_addr);
        self.eclipse_guard.write().await.unregister(peer_addr.ip());
        Ok(())
    }

    async fn dispatch_message(
        self: &Arc<Self>,
        peer_addr: SocketAddr,
        reply_tx: &mpsc::Sender<WireMessage>,
        msg: WireMessage,
    ) {
        match msg.command.as_str() {
            "ping" => {
                let _ = reply_tx
                    .send(WireMessage::new(self.magic, "pong", msg.payload))
                    .await;
            }
            "pong" => {
                if msg.payload.len() == 8 {
                    let nonce = u64::from_le_bytes(msg.payload.try_into().unwrap());
                    if let Some(sent_ms) = self.pending_pings.write().await.remove(&(peer_addr, nonce)) {
                        let now_ms = current_unix_millis();
                        if let Some(peer) = self.peers.write().await.get_mut(&peer_addr) {
                            peer.latency_ms = now_ms.saturating_sub(sent_ms);
                        }
                    }
                }
            }
            "getaddr" => {
                let known: Vec<String> = self
                    .known_peers
                    .read()
                    .await
                    .values()
                    .filter(|r| r.banned_until <= current_unix_timestamp())
                    .take(32)
                    .map(|r| r.addr.clone())
                    .collect();
                let _ = reply_tx
                    .send(WireMessage::new(
                        self.magic,
                        "addr",
                        serde_json::to_vec(&known).unwrap(),
                    ))
                    .await;
            }
            "addr" => {
                if let Ok(addrs) = serde_json::from_slice::<Vec<String>>(&msg.payload) {
                    let now = current_unix_timestamp();
                    {
                        let mut known = self.known_peers.write().await;
                        for a in addrs.iter().take(32) {
                            if known.len() < 256 {
                                known.entry(a.clone()).or_insert(PeerRecord {
                                    addr: a.clone(),
                                    score: 100,
                                    last_seen: now,
                                    banned_until: 0,
                                });
                            }
                        }
                    }
                    self.persist_peers().await;
                }
            }
            "getheaders" => {
                if let Ok(locator) = serde_json::from_slice::<Vec<String>>(&msg.payload) {
                    let headers_bytes = {
                        let l = self.ledger.read().await;
                        let mut start_height = 0u64;
                        for hex_hash in locator {
                            if let Ok(bytes) = hex::decode(&hex_hash) {
                                if bytes.len() == 32 {
                                    let mut arr = [0u8; 32];
                                    arr.copy_from_slice(&bytes);
                                    if let Some(node) = l.blocks.get(&arr) {
                                        if node.on_main_chain {
                                            start_height = node.block.header.block_height;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        let mut out = Vec::new();
                        let end = (start_height + 100).min(l.best_height);
                        for h in (start_height + 1)..=end {
                            if let Some(node) = l.get_block_by_height(h) {
                                out.push(node.block.header.to_bytes().to_vec());
                            }
                        }
                        out
                    };
                    let _ = reply_tx
                        .send(WireMessage::new(
                            self.magic,
                            "headers",
                            serde_json::to_vec(&headers_bytes).unwrap(),
                        ))
                        .await;
                }
            }
            "headers" => {
                if let Ok(hdr_list) = serde_json::from_slice::<Vec<Vec<u8>>>(&msg.payload) {
                    let mut to_fetch = Vec::new();
                    {
                        let l = self.ledger.read().await;
                        for raw in hdr_list.into_iter().take(100) {
                            if raw.len() == BLOCK_HEADER_SIZE {
                                if let Ok(hdr) = BlockHeader::from_bytes(&raw) {
                                    let hash = hdr.block_hash();
                                    if !l.blocks.contains_key(&hash) {
                                        to_fetch.push(InvVector {
                                            kind: InvKind::Block,
                                            hash_hex: hex::encode(hash),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    if !to_fetch.is_empty() {
                        let _ = reply_tx
                            .send(WireMessage::new(
                                self.magic,
                                "getdata",
                                serde_json::to_vec(&to_fetch).unwrap(),
                            ))
                            .await;
                    }
                }
            }
            "inv" => {
                if let Ok(items) = serde_json::from_slice::<Vec<InvVector>>(&msg.payload) {
                    let mut want = Vec::new();
                    let l = self.ledger.read().await;
                    let m = self.mempool.read().await;
                    for item in items.into_iter().take(128) {
                        if let Ok(bytes) = hex::decode(&item.hash_hex) {
                            if bytes.len() == 32 {
                                let mut arr = [0u8; 32];
                                arr.copy_from_slice(&bytes);
                                match item.kind {
                                    InvKind::Block => {
                                        if !l.blocks.contains_key(&arr) {
                                            want.push(item);
                                        }
                                    }
                                    InvKind::Tx => {
                                        if !m.entries.contains_key(&arr)
                                            && !l.tx_index.contains_key(&arr)
                                        {
                                            want.push(item);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    drop(m);
                    drop(l);
                    if !want.is_empty() {
                        let _ = reply_tx
                            .send(WireMessage::new(
                                self.magic,
                                "getdata",
                                serde_json::to_vec(&want).unwrap(),
                            ))
                            .await;
                    }
                }
            }
            "getdata" => {
                if let Ok(items) = serde_json::from_slice::<Vec<InvVector>>(&msg.payload) {
                    for item in items.into_iter().take(64) {
                        if let Ok(bytes) = hex::decode(&item.hash_hex) {
                            if bytes.len() == 32 {
                                let mut arr = [0u8; 32];
                                arr.copy_from_slice(&bytes);
                                match item.kind {
                                    InvKind::Block => {
                                        let blk_bytes = {
                                            let l = self.ledger.read().await;
                                            l.blocks.get(&arr).map(|n| n.block.to_bytes())
                                        };
                                        if let Some(raw) = blk_bytes {
                                            let _ = reply_tx
                                                .send(WireMessage::new(self.magic, "block", raw))
                                                .await;
                                        }
                                    }
                                    InvKind::Tx => {
                                        let tx_bytes = {
                                            let m = self.mempool.read().await;
                                            m.entries.get(&arr).map(|e| e.tx.to_bytes())
                                        };
                                        if let Some(raw) = tx_bytes {
                                            let _ = reply_tx
                                                .send(WireMessage::new(self.magic, "tx", raw))
                                                .await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "block" => {
                match Block::from_bytes(&msg.payload) {
                    Ok(block) => {
                        let block_hash = block.block_hash();
                        let block_height = block.header.block_height;
                        let now = current_unix_timestamp();
                        let apply_res = {
                            let mut l = self.ledger.write().await;
                            l.submit_block_at(block.clone(), now)
                        };
                        match apply_res {
                            Ok(_) => {
                                {
                                    let l = self.ledger.read().await;
                                    let mut m = self.mempool.write().await;
                                    m.prune_against_ledger(&l);
                                }
                                if let Some(p) = self.peers.write().await.get_mut(&peer_addr) {
                                    p.best_height = p.best_height.max(block_height);
                                }
                                // Relay announcement to other peers
                                self.relay_inv(
                                    Some(peer_addr),
                                    vec![InvVector {
                                        kind: InvKind::Block,
                                        hash_hex: hex::encode(block_hash),
                                    }],
                                )
                                .await;
                            }
                            Err(LedgerError::DuplicateBlock(_)) => {}
                            Err(LedgerError::PrevBlockMismatch(_)) => {
                                // Missing ancestor: request headers from peer to sync chain
                                let locator = self.build_locator().await;
                                let _ = reply_tx
                                    .send(WireMessage::new(
                                        self.magic,
                                        "getheaders",
                                        serde_json::to_vec(&locator).unwrap(),
                                    ))
                                    .await;
                            }
                            Err(e) => {
                                log::warn!("Invalid block from {}: {}", peer_addr, e);
                                self.penalize_peer(peer_addr, 50).await;
                            }
                        }
                    }
                    Err(_) => {
                        self.penalize_peer(peer_addr, 50).await;
                    }
                }
            }
            "tx" => {
                match Transaction::from_bytes(&msg.payload) {
                    Ok(tx) => {
                        let now = current_unix_timestamp();
                        let peer_key = peer_addr.ip().to_string();
                        let txid = tx.txid();
                        let insert_res = {
                            let l = self.ledger.read().await;
                            let mut m = self.mempool.write().await;
                            m.insert_transaction_with_ledger(tx, &l, Some(&peer_key), now)
                        };
                        match insert_res {
                            Ok(_) => {
                                self.relay_inv(
                                    Some(peer_addr),
                                    vec![InvVector {
                                        kind: InvKind::Tx,
                                        hash_hex: hex::encode(txid),
                                    }],
                                )
                                .await;
                            }
                            Err(MempoolError::DuplicateTransaction) => {}
                            Err(MempoolError::RateLimited) => {
                                self.penalize_peer(peer_addr, 10).await;
                            }
                            Err(e) => {
                                log::debug!("Rejected tx from {}: {}", peer_addr, e);
                                self.penalize_peer(peer_addr, 15).await;
                            }
                        }
                    }
                    Err(_) => {
                        self.penalize_peer(peer_addr, 25).await;
                    }
                }
            }
            _ => {
                self.penalize_peer(peer_addr, 10).await;
            }
        }
    }

    async fn build_locator(&self) -> Vec<String> {
        let l = self.ledger.read().await;
        let mut locator = Vec::new();
        let mut step = 1usize;
        let mut idx = l.main_chain.len().saturating_sub(1);
        loop {
            locator.push(hex::encode(l.main_chain[idx]));
            if idx == 0 || locator.len() >= 16 {
                break;
            }
            idx = idx.saturating_sub(step);
            if locator.len() > 6 {
                step *= 2;
            }
        }
        locator
    }

    async fn sync_with_peers(&self) {
        let our_height = self.ledger.read().await.best_height;
        let locator = self.build_locator().await;
        let nonce = rand::rngs::OsRng.next_u64();
        let now_ms = current_unix_millis();

        let peer_senders: Vec<(SocketAddr, u64, mpsc::Sender<WireMessage>)> = self
            .peers
            .read()
            .await
            .iter()
            .map(|(addr, p)| (*addr, p.best_height, p.tx_sender.clone()))
            .collect();

        for (addr, peer_h, sender) in peer_senders {
            self.pending_pings
                .write()
                .await
                .insert((addr, nonce), now_ms);
            let _ = sender
                .send(WireMessage::new(
                    self.magic,
                    "ping",
                    nonce.to_le_bytes().to_vec(),
                ))
                .await;

            // Always poll getheaders if peer claims higher height, or periodically for partition healing
            if peer_h > our_height || (nonce % 3 == 0) {
                let _ = sender
                    .send(WireMessage::new(
                        self.magic,
                        "getheaders",
                        serde_json::to_vec(&locator).unwrap(),
                    ))
                    .await;
            }
        }
    }

    async fn relay_inv(&self, exclude: Option<SocketAddr>, inv: Vec<InvVector>) {
        let payload = serde_json::to_vec(&inv).unwrap();
        let msg = WireMessage::new(self.magic, "inv", payload);
        let senders: Vec<mpsc::Sender<WireMessage>> = self
            .peers
            .read()
            .await
            .iter()
            .filter(|(addr, _)| Some(**addr) != exclude)
            .map(|(_, p)| p.tx_sender.clone())
            .collect();
        for sender in senders {
            let _ = sender.send(msg.clone()).await;
        }
    }

    /// Broadcast a newly mined/staked block immediately to all connected peers
    pub async fn broadcast_block(&self, block: &Block) {
        let msg = WireMessage::new(self.magic, "block", block.to_bytes());
        let height = block.header.block_height;
        let senders: Vec<(SocketAddr, mpsc::Sender<WireMessage>)> = self
            .peers
            .read()
            .await
            .iter()
            .map(|(a, p)| (*a, p.tx_sender.clone()))
            .collect();
        for (_addr, sender) in senders {
            let _ = sender.send(msg.clone()).await;
        }
        let mut peers = self.peers.write().await;
        for p in peers.values_mut() {
            if p.best_height < height {
                p.best_height = height;
            }
        }
    }

    /// Broadcast a locally submitted transaction to all connected peers
    pub async fn broadcast_tx(&self, tx: &Transaction) {
        let msg = WireMessage::new(self.magic, "tx", tx.to_bytes());
        let senders: Vec<mpsc::Sender<WireMessage>> = self
            .peers
            .read()
            .await
            .values()
            .map(|p| p.tx_sender.clone())
            .collect();
        for sender in senders {
            let _ = sender.send(msg.clone()).await;
        }
    }

    async fn penalize_peer(&self, peer_addr: SocketAddr, penalty: i32) {
        let mut should_ban = false;
        if let Some(peer) = self.peers.write().await.get_mut(&peer_addr) {
            peer.score -= penalty;
            if peer.score <= 0 {
                should_ban = true;
            }
        }
        if should_ban {
            let until = current_unix_timestamp() + BAN_DURATION_SECS;
            self.banned_ips.write().await.insert(peer_addr.ip(), until);
        }
    }

    pub async fn get_peer_snapshots(&self) -> Vec<PeerSnapshot> {
        self.peers
            .read()
            .await
            .values()
            .map(|p| PeerSnapshot {
                addr: p.addr.to_string(),
                listen_addr: p.listen_addr.clone(),
                inbound: p.inbound,
                best_height: p.best_height,
                score: p.score,
                user_agent: p.user_agent.clone(),
                latency_ms: p.latency_ms,
                connected_since: p.connected_since,
            })
            .collect()
    }

    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    pub async fn known_peer_count(&self) -> usize {
        self.known_peers.read().await.len()
    }
}

async fn read_message_half(
    stream: &mut tokio::net::tcp::OwnedReadHalf,
    expected_magic: u32,
) -> Result<WireMessage, P2pError> {
    use tokio::io::AsyncReadExt;
    let mut header_buf = [0u8; 24];
    stream
        .read_exact(&mut header_buf)
        .await
        .map_err(|e| P2pError::Io(e.to_string()))?;

    let magic = u32::from_le_bytes(header_buf[0..4].try_into().unwrap());
    if magic != expected_magic {
        return Err(P2pError::InvalidMagic(magic));
    }
    let command = String::from_utf8_lossy(&header_buf[4..16])
        .trim_matches('\0')
        .to_string();
    let payload_len = u32::from_le_bytes(header_buf[16..20].try_into().unwrap()) as usize;
    let mut checksum = [0u8; 4];
    checksum.copy_from_slice(&header_buf[20..24]);

    if payload_len > 8 * 1024 * 1024 {
        return Err(P2pError::PayloadTooLarge);
    }

    let mut payload = vec![0u8; payload_len];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(|e| P2pError::Io(e.to_string()))?;

    let expected_hash = hash_blake3(&payload);
    if checksum != expected_hash[0..4] {
        return Err(P2pError::ChecksumMismatch);
    }

    Ok(WireMessage {
        magic,
        command,
        payload,
    })
}

fn current_unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdr_primitives::Network;

    #[test]
    fn test_wire_framing_roundtrip() {
        let payload = b"version_payload_test_data".to_vec();
        let msg = WireMessage::new(SARDHAN_MAINNET_MAGIC, "version", payload.clone());
        let wire_bytes = msg.to_bytes();

        let decoded = WireMessage::from_bytes(&wire_bytes).expect("framing decode succeeds");
        assert_eq!(decoded.command, "version");
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_eclipse_guard_subnet_16_limit() {
        let mut guard = EclipseGuard::default();
        let ip1: IpAddr = "203.0.113.10".parse().unwrap();
        let ip2: IpAddr = "203.0.113.20".parse().unwrap();
        let ip3: IpAddr = "203.0.113.30".parse().unwrap();
        let other_subnet: IpAddr = "198.51.100.10".parse().unwrap();

        assert!(guard.can_accept(ip1));
        guard.register(ip1);
        assert!(guard.can_accept(ip2));
        guard.register(ip2);
        // 3rd peer from same /16 must be rejected
        assert!(!guard.can_accept(ip3));
        assert!(guard.can_accept(other_subnet));
    }

    #[tokio::test]
    async fn test_tcp_p2p_handshake_live_exchange() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            P2pNetwork::perform_handshake(&mut socket, SARDHAN_MAINNET_MAGIC, 100).await
        });

        let mut client_socket = TcpStream::connect(addr).await.unwrap();
        let client_res =
            P2pNetwork::perform_handshake(&mut client_socket, SARDHAN_MAINNET_MAGIC, 50).await;

        let server_res = server_handle.await.unwrap();
        assert_eq!(client_res.unwrap(), 100);
        assert_eq!(server_res.unwrap(), 50);
    }

    #[tokio::test]
    async fn test_multi_node_p2p_sync_and_partition_recovery() {
        let l1 = Arc::new(RwLock::new(Ledger::new_for_network(Network::Regtest)));
        let m1 = Arc::new(RwLock::new(Mempool::new(SARDHAN_REGTEST_MAGIC)));
        let l2 = Arc::new(RwLock::new(Ledger::new_for_network(Network::Regtest)));
        let m2 = Arc::new(RwLock::new(Mempool::new(SARDHAN_REGTEST_MAGIC)));

        let p2p1 = P2pManager::new(
            SARDHAN_REGTEST_MAGIC,
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&l1),
            m1,
            None,
        );
        let p2p2 = P2pManager::new(
            SARDHAN_REGTEST_MAGIC,
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&l2),
            m2,
            None,
        );

        let _a1 = p2p1.start().await.unwrap();
        let a2 = p2p2.start().await.unwrap();
        p2p1.connect_peer(&a2.to_string()).await.unwrap();

        // Wait briefly for handshake
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(p2p1.peer_count().await >= 1);
        assert!(p2p2.peer_count().await >= 1);

        // Mine 2 blocks on Node 1 while partitioned from Node 2
        p2p1.disconnect_all().await;
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(p2p1.peer_count().await, 0);

        for _ in 0..2 {
            let blk = mine_test_block(&*l1.read().await);
            l1.write().await.apply_block(&blk.header, &blk.transactions).unwrap();
        }
        assert_eq!(l1.read().await.best_height, 2);
        assert_eq!(l2.read().await.best_height, 0);

        // Reconnect after partition -> automatic header/block sync heals Node 2 to height 2
        p2p1.connect_peer(&a2.to_string()).await.unwrap();
        for _ in 0..30 {
            if l2.read().await.best_height == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(l2.read().await.best_height, 2);
        assert_eq!(
            l2.read().await.best_block_hash,
            l1.read().await.best_block_hash
        );
    }

    fn mine_test_block(ledger: &Ledger) -> Block {
        let h = ledger.best_height + 1;
        let coinbase = Transaction {
            chain_id: ledger.config.chain_id,
            version: 1,
            recent_block_hash: ledger.best_block_hash,
            micro_pow_nonce: 0,
            lock_time: 0,
            inputs: vec![],
            outputs: vec![sdr_primitives::TxOut {
                value: sdr_consensus::calculate_block_reward(h),
                recipient_pubkey: [0x11; 32],
            }],
            witnesses: vec![],
        };
        let bits = ledger.expected_next_bits(ledger.best_block_hash, sdr_primitives::ConsensusType::ProofOfWork);
        let lsm = ledger.expected_lagging_modifier(ledger.best_block_hash, h);
        let parent_ts = ledger.blocks[&ledger.best_block_hash].block.header.timestamp;
        let mut header = BlockHeader {
            version: 1,
            prev_block_hash: ledger.best_block_hash,
            merkle_root: sdr_primitives::compute_merkle_root(&[coinbase.txid()]),
            timestamp: parent_ts + 10,
            block_height: h,
            consensus_bits: bits,
            consensus_type: sdr_primitives::ConsensusType::ProofOfWork,
            lagging_stake_modifier: lsm,
            pow_nonce: 0,
            reserved_flags: [0; 3],
            validator_signature: vec![0; 64],
        };
        let target = sdr_consensus::compact_to_target(bits);
        for n in 1..10_000u64 {
            header.pow_nonce = n;
            if sdr_consensus::hash_less_than_target(&sdr_crypto::sardhan_hash(&header.to_bytes()), &target) {
                return Block {
                    header,
                    transactions: vec![coinbase],
                };
            }
        }
        panic!("regtest mine failed");
    }
}
