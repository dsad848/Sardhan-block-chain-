import express, { Request, Response } from 'express';
import { spawn, ChildProcess, execFile } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PORT = Number(process.env.PORT || 3000);
const DATA_ROOT = path.resolve(__dirname, '.sdr-testnet');
const RUST_TARGET_RELEASE = path.resolve(__dirname, 'sardhan-core/target/release');
const RUST_TARGET_DEBUG = path.resolve(__dirname, 'sardhan-core/target/debug');

interface NodeDescriptor {
  index: number;
  id: string;
  name: string;
  role: string;
  p2pPort: number;
  rpcPort: number;
  datadir: string;
  extraArgs: string[];
}

const TESTNET_NODES: NodeDescriptor[] = [
  {
    index: 1,
    id: 'node-01',
    name: 'sdrd-seed-01',
    role: 'Seed Validator + CPU Miner',
    p2pPort: 19333,
    rpcPort: 19332,
    datadir: path.join(DATA_ROOT, 'node1'),
    extraArgs: ['--mine', '--mine-threads', '1', '--mine-interval-ms', '1500'],
  },
  {
    index: 2,
    id: 'node-02',
    name: 'sdrd-miner-02',
    role: 'CPU Miner Peer',
    p2pPort: 19343,
    rpcPort: 19342,
    datadir: path.join(DATA_ROOT, 'node2'),
    extraArgs: [
      '--addnode',
      '127.0.0.1:19333',
      '--mine',
      '--mine-threads',
      '1',
      '--mine-interval-ms',
      '2000',
    ],
  },
  {
    index: 3,
    id: 'node-03',
    name: 'sdrd-staker-03',
    role: 'Community PoS Staker + Peer',
    p2pPort: 19353,
    rpcPort: 19352,
    datadir: path.join(DATA_ROOT, 'node3'),
    extraArgs: [
      '--addnode',
      '127.0.0.1:19333',
      '--addnode',
      '127.0.0.1:19343',
      '--mine',
      '--mine-threads',
      '1',
      '--mine-interval-ms',
      '3500',
      '--stake',
    ],
  },
];

const ALLOWED_RPC_METHODS = new Set([
  'getblockchaininfo',
  'getblock',
  'getlatestblocks',
  'getrawtransaction',
  'sendrawtransaction',
  'getaddressinfo',
  'getmempoolinfo',
  'getrawmempool',
  'getmininginfo',
  'getstakinginfo',
  'getpeerinfo',
  'getnetworkinfo',
  'getnodehealth',
  'addnode',
  'disconnectnode',
]);

const managedProcesses: ChildProcess[] = [];
const nodeLogs: Record<number, string[]> = { 1: [], 2: [], 3: [] };
let lastBenchmarkCache: { timestamp: string; output: string } | null = null;
let miningOnceInFlight = false;

function resolveBinary(name: string): string | null {
  const rel = path.join(RUST_TARGET_RELEASE, name);
  if (fs.existsSync(rel)) return rel;
  const dbg = path.join(RUST_TARGET_DEBUG, name);
  if (fs.existsSync(dbg)) return dbg;
  return null;
}

function appendNodeLog(index: number, chunk: string) {
  const lines = chunk
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
  if (lines.length === 0) return;
  const arr = nodeLogs[index] ?? [];
  arr.push(...lines);
  nodeLogs[index] = arr.slice(-40);
}

async function callNodeRpc(rpcPort: number, method: string, params: unknown[] = []): Promise<unknown> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 3500);
  try {
    const resp = await fetch(`http://127.0.0.1:${rpcPort}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
      signal: controller.signal,
    });
    const payload = (await resp.json()) as { result?: unknown; error?: { code: number; message: string } };
    if (payload.error) {
      throw new Error(`RPC ${method} error (${payload.error.code}): ${payload.error.message}`);
    }
    return payload.result;
  } finally {
    clearTimeout(timer);
  }
}

async function isNodeAlive(rpcPort: number): Promise<boolean> {
  try {
    await callNodeRpc(rpcPort, 'getblockchaininfo', []);
    return true;
  } catch {
    return false;
  }
}

async function ensureThreeNodeTestnet() {
  const sdrdPath = resolveBinary('sdrd');
  if (!sdrdPath) {
    console.warn('[Sardhan Cluster] `sdrd` binary not found in sardhan-core/target/{release,debug}.');
    return;
  }
  fs.mkdirSync(DATA_ROOT, { recursive: true });

  for (const node of TESTNET_NODES) {
    if (await isNodeAlive(node.rpcPort)) {
      console.log(`[Sardhan Cluster] Reusing live ${node.name} on RPC ${node.rpcPort}`);
      continue;
    }

    fs.mkdirSync(node.datadir, { recursive: true });
    const args = [
      '--network',
      'testnet',
      '--datadir',
      node.datadir,
      '--bind',
      '127.0.0.1',
      '--p2p-port',
      String(node.p2pPort),
      '--rpc-port',
      String(node.rpcPort),
      ...node.extraArgs,
    ];

    const child = spawn(sdrdPath, args, {
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    managedProcesses.push(child);

    child.stdout?.on('data', (d) => appendNodeLog(node.index, d.toString()));
    child.stderr?.on('data', (d) => appendNodeLog(node.index, d.toString()));
    child.on('exit', (code) => {
      appendNodeLog(node.index, `[EXIT] Process exited with code ${code}`);
    });

    // Small staggered startup so seed node binds before peers connect
    await new Promise((r) => setTimeout(r, 350));
  }
}

function shutdownCluster() {
  for (const proc of managedProcesses) {
    if (!proc.killed) {
      proc.kill('SIGTERM');
    }
  }
}

process.on('SIGINT', () => {
  shutdownCluster();
  process.exit(0);
});
process.on('SIGTERM', () => {
  shutdownCluster();
  process.exit(0);
});
process.on('exit', shutdownCluster);

async function startServer() {
  await ensureThreeNodeTestnet();

  const app = express();
  app.use(express.json({ limit: '2mb' }));

  app.get('/api/cluster/status', async (_req: Request, res: Response) => {
    const nodeStatuses = await Promise.all(
      TESTNET_NODES.map(async (node) => {
        try {
          const [info, health, peers, network, mining, staking, mempoolInfo] = await Promise.all([
            callNodeRpc(node.rpcPort, 'getblockchaininfo'),
            callNodeRpc(node.rpcPort, 'getnodehealth'),
            callNodeRpc(node.rpcPort, 'getpeerinfo'),
            callNodeRpc(node.rpcPort, 'getnetworkinfo'),
            callNodeRpc(node.rpcPort, 'getmininginfo'),
            callNodeRpc(node.rpcPort, 'getstakinginfo'),
            callNodeRpc(node.rpcPort, 'getmempoolinfo'),
          ]);
          return {
            index: node.index,
            id: node.id,
            name: node.name,
            role: node.role,
            p2pPort: node.p2pPort,
            rpcPort: node.rpcPort,
            online: true,
            info,
            health,
            peers,
            network,
            mining,
            staking,
            mempoolInfo,
            logs: nodeLogs[node.index] ?? [],
          };
        } catch (err) {
          return {
            index: node.index,
            id: node.id,
            name: node.name,
            role: node.role,
            p2pPort: node.p2pPort,
            rpcPort: node.rpcPort,
            online: false,
            error: err instanceof Error ? err.message : String(err),
            logs: nodeLogs[node.index] ?? [],
          };
        }
      })
    );

    const primary = nodeStatuses.find((n) => n.online) ?? nodeStatuses[0];
    let latestBlocks: unknown[] = [];
    let rawMempool: unknown[] = [];
    if (primary && primary.online) {
      try {
        const [blks, mpool] = await Promise.all([
          callNodeRpc(primary.rpcPort, 'getlatestblocks', [25]),
          callNodeRpc(primary.rpcPort, 'getrawmempool', []),
        ]);
        if (Array.isArray(blks)) latestBlocks = blks;
        if (Array.isArray(mpool)) rawMempool = mpool;
      } catch {
        // Ignore transient query errors
      }
    }

    res.json({
      network: 'sdr-testnet',
      timestamp: new Date().toISOString(),
      nodes: nodeStatuses,
      latestBlocks,
      rawMempool,
    });
  });

  app.post('/api/rpc/:nodeIndex', async (req: Request, res: Response) => {
    const idx = Number(req.params.nodeIndex);
    const node = TESTNET_NODES.find((n) => n.index === idx);
    if (!node) {
      res.status(404).json({ error: 'Unknown node index (expected 1, 2, or 3)' });
      return;
    }

    const { method, params } = req.body as { method?: string; params?: unknown[] };
    if (!method || !ALLOWED_RPC_METHODS.has(method)) {
      res.status(400).json({ error: `Unsupported or disallowed RPC method '${method}'` });
      return;
    }

    try {
      const result = await callNodeRpc(node.rpcPort, method, Array.isArray(params) ? params : []);
      res.json({ jsonrpc: '2.0', id: 1, result });
    } catch (err) {
      res.status(502).json({
        jsonrpc: '2.0',
        id: 1,
        error: { code: -32000, message: err instanceof Error ? err.message : String(err) },
      });
    }
  });

  app.post('/api/cluster/partition', async (_req: Request, res: Response) => {
    try {
      const result = await callNodeRpc(19352, 'disconnectnode', ['all']);
      res.json({ ok: true, node: 'sdrd-staker-03', result });
    } catch (err) {
      res.status(502).json({ ok: false, error: err instanceof Error ? err.message : String(err) });
    }
  });

  app.post('/api/cluster/reconnect', async (_req: Request, res: Response) => {
    try {
      const r1 = await callNodeRpc(19352, 'addnode', ['127.0.0.1:19333']);
      const r2 = await callNodeRpc(19352, 'addnode', ['127.0.0.1:19343']);
      res.json({ ok: true, node: 'sdrd-staker-03', results: [r1, r2] });
    } catch (err) {
      res.status(502).json({ ok: false, error: err instanceof Error ? err.message : String(err) });
    }
  });

  app.post('/api/cluster/mine-reward', async (req: Request, res: Response) => {
    const { address } = req.body as { address?: string };
    if (!address || !address.startsWith('tsdr1')) {
      res.status(400).json({ error: 'Valid testnet address (tsdr1...) is required' });
      return;
    }
    if (miningOnceInFlight) {
      res.status(429).json({ error: 'A PoW mining job is already in progress; wait a second and retry' });
      return;
    }
    const minerPath = resolveBinary('sdr-miner');
    if (!minerPath) {
      res.status(500).json({ error: 'sdr-miner binary unavailable' });
      return;
    }

    miningOnceInFlight = true;
    execFile(
      minerPath,
      ['--rpc', '127.0.0.1:19332', '--address', address, '--threads', '2', '--blocks', '1'],
      { timeout: 30_000 },
      (err, stdout, stderr) => {
        miningOnceInFlight = false;
        if (err) {
          res.status(500).json({
            ok: false,
            error: stderr || err.message,
            output: stdout,
          });
          return;
        }
        res.json({ ok: true, output: stdout });
      }
    );
  });

  app.get('/api/miner/benchmark', async (req: Request, res: Response) => {
    const force = req.query.refresh === '1';
    if (lastBenchmarkCache && !force) {
      res.json(lastBenchmarkCache);
      return;
    }
    const minerPath = resolveBinary('sdr-miner');
    if (!minerPath) {
      res.status(500).json({ error: 'sdr-miner binary unavailable' });
      return;
    }
    execFile(minerPath, ['--benchmark', '1', '--threads', '2'], { timeout: 25_000 }, (err, stdout, stderr) => {
      if (err) {
        res.status(500).json({ error: stderr || err.message });
        return;
      }
      lastBenchmarkCache = {
        timestamp: new Date().toISOString(),
        output: stdout,
      };
      res.json(lastBenchmarkCache);
    });
  });

  const distDir = path.resolve(__dirname, 'dist');
  const isProd = process.env.NODE_ENV === 'production' && fs.existsSync(distDir);

  if (isProd) {
    app.use(express.static(distDir));
    app.get('*', (_req, res) => {
      res.sendFile(path.join(distDir, 'index.html'));
    });
  } else {
    const { createServer: createViteServer } = await import('vite');
    const vite = await createViteServer({
      server: { middlewareMode: true },
      appType: 'spa',
    });
    app.use(vite.middlewares);
  }

  app.listen(PORT, '0.0.0.0', () => {
    console.log(`[Sardhan Explorer & Node Gateway] Listening on http://0.0.0.0:${PORT}`);
  });
}

startServer().catch((err) => {
  console.error('Fatal server startup failure:', err);
  process.exit(1);
});
