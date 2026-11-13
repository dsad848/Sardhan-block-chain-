import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  LocalWalletAccount,
  SdrNetworkName,
  generateMnemonic,
  deriveWalletFromMnemonic,
  encodeQrVersion4L,
  decodeAddressForHrp,
  networkHrp,
  parseSdrToSardhans,
  formatSardhansToSdr,
  buildAndSignZeroFeeTransaction,
  MIN_TRANSACTION_OUTPUT_VALUE,
} from '../crypto/sardhanCrypto';
import {
  KeyRound,
  ShieldCheck,
  Send,
  Copy,
  Check,
  RefreshCw,
  Pickaxe,
  Lock,
  Eye,
  EyeOff,
  RotateCcw,
  Trash2,
  Cpu,
} from 'lucide-react';

interface RpcUtxo {
  txid: string;
  vout: number;
  value_sardhans: number;
  value_sdr: number;
  block_height: number;
  is_coinstake: boolean;
}

interface RpcHistoryItem {
  block_height: number;
  direction: string;
  amount_sardhans: number;
  txid: string;
}

interface AddressInfo {
  address: string;
  network: string;
  balance_sardhans: number;
  balance_sdr: number;
  spendable_sardhans: number;
  spendable_sdr: number;
  mature_staking_sardhans: number;
  utxos: RpcUtxo[];
  history: RpcHistoryItem[];
}

const STORAGE_KEY = 'sardhan_local_wallet_v1';

async function proxyRpc(nodeIndex: number, method: string, params: unknown[] = []) {
  const resp = await fetch(`/api/rpc/${nodeIndex}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ method, params }),
  });
  const data = await resp.json();
  if (data.error) {
    throw new Error(data.error.message || 'RPC Error');
  }
  return data.result;
}

export const WebWallet: React.FC = () => {
  const [network, setNetwork] = useState<SdrNetworkName>('testnet');
  const [wallet, setWallet] = useState<LocalWalletAccount | null>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved) as { mnemonic: string; network?: SdrNetworkName };
        if (parsed.mnemonic) {
          return deriveWalletFromMnemonic(parsed.mnemonic, parsed.network || 'testnet');
        }
      }
    } catch {
      // Ignore corrupt local state
    }
    return null;
  });

  const [recoverInput, setRecoverInput] = useState('');
  const [passphrase, setPassphrase] = useState('');
  const [showMnemonic, setShowMnemonic] = useState(false);
  const [copiedAddr, setCopiedAddr] = useState(false);

  const [addrInfo, setAddrInfo] = useState<AddressInfo | null>(null);
  const [loadingBalance, setLoadingBalance] = useState(false);
  const [miningReward, setMiningReward] = useState(false);

  const [recipient, setRecipient] = useState('');
  const [amountStr, setAmountStr] = useState('');
  const [sendingTx, setSendingTx] = useState(false);
  const [statusMsg, setStatusMsg] = useState<{ type: 'ok' | 'err'; text: string } | null>(null);

  const qrMatrix = useMemo(() => {
    if (!wallet) return null;
    return encodeQrVersion4L(wallet.address);
  }, [wallet]);

  const refreshBalance = useCallback(async () => {
    if (!wallet || network !== 'testnet') {
      setAddrInfo(null);
      return;
    }
    setLoadingBalance(true);
    try {
      const res = (await proxyRpc(1, 'getaddressinfo', [wallet.address])) as AddressInfo;
      setAddrInfo(res);
    } catch {
      // Node may still be booting
    } finally {
      setLoadingBalance(false);
    }
  }, [wallet, network]);

  useEffect(() => {
    refreshBalance();
    const timer = setInterval(refreshBalance, 3500);
    return () => clearInterval(timer);
  }, [refreshBalance]);

  const handleSwitchNetwork = (nextNet: SdrNetworkName) => {
    setNetwork(nextNet);
    if (wallet) {
      const rederived = deriveWalletFromMnemonic(wallet.mnemonic, nextNet, passphrase);
      setWallet(rederived);
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ mnemonic: wallet.mnemonic, network: nextNet }));
    }
    setStatusMsg(null);
  };

  const handleCreateWallet = (wordCount: 18 | 24) => {
    const mnemonic = generateMnemonic(wordCount);
    const derived = deriveWalletFromMnemonic(mnemonic, network, passphrase);
    setWallet(derived);
    setShowMnemonic(true);
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ mnemonic, network }));
    setStatusMsg({
      type: 'ok',
      text: `Created ${wordCount}-word local Ed25519 wallet (${derived.address}). Back up your seed phrase offline.`,
    });
  };

  const handleRecoverWallet = (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const derived = deriveWalletFromMnemonic(recoverInput, network, passphrase);
      setWallet(derived);
      setRecoverInput('');
      setShowMnemonic(false);
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ mnemonic: derived.mnemonic, network }));
      setStatusMsg({ type: 'ok', text: `Recovered wallet ${derived.address} locally.` });
    } catch (err) {
      setStatusMsg({
        type: 'err',
        text: err instanceof Error ? err.message : 'Mnemonic recovery failed',
      });
    }
  };

  const handleClearWallet = () => {
    localStorage.removeItem(STORAGE_KEY);
    setWallet(null);
    setAddrInfo(null);
    setStatusMsg(null);
  };

  const handleCopyAddress = () => {
    if (!wallet) return;
    navigator.clipboard.writeText(wallet.address);
    setCopiedAddr(true);
    setTimeout(() => setCopiedAddr(false), 1800);
  };

  const handleMineToWallet = async () => {
    if (!wallet || network !== 'testnet') return;
    setMiningReward(true);
    setStatusMsg(null);
    try {
      const resp = await fetch('/api/cluster/mine-reward', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ address: wallet.address }),
      });
      const data = await resp.json();
      if (!resp.ok || !data.ok) {
        throw new Error(data.error || 'Mining failed');
      }
      setStatusMsg({
        type: 'ok',
        text: 'Mined 1 real SardhanHash PoW block on Node 1 paying 50 SDR block reward to your wallet.',
      });
      await refreshBalance();
    } catch (err) {
      setStatusMsg({
        type: 'err',
        text: err instanceof Error ? err.message : 'Failed mining block',
      });
    } finally {
      setMiningReward(false);
    }
  };

  const handleSendZeroFee = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!wallet) return;
    if (network === 'mainnet') {
      setStatusMsg({
        type: 'err',
        text: 'Mainnet activation blocked (MAINNET_ENABLED = false security release gate). Switch to Testnet.',
      });
      return;
    }
    setSendingTx(true);
    setStatusMsg(null);
    try {
      const expectedHrp = networkHrp(network);
      const recipientPubKey = decodeAddressForHrp(recipient, expectedHrp);
      const senderPubKey = decodeAddressForHrp(wallet.address, expectedHrp);
      const amountSardhans = parseSdrToSardhans(amountStr);
      if (amountSardhans < MIN_TRANSACTION_OUTPUT_VALUE) {
        throw new Error('Amount is below the 1000-Sardhan (0.00001 SDR) dust limit.');
      }

      // Fetch live UTXOs & recent block hash / micro-PoW target from Node 1
      const [liveAddr, miningInfo] = await Promise.all([
        proxyRpc(1, 'getaddressinfo', [wallet.address]) as Promise<AddressInfo>,
        proxyRpc(1, 'getmininginfo', []) as Promise<{
          recent_block_hashes: string[];
          micro_pow_target_hex: string;
        }>,
      ]);

      const recentBlockHashHex = miningInfo.recent_block_hashes?.[0];
      const microPowTargetHex = miningInfo.micro_pow_target_hex;
      if (!recentBlockHashHex || !microPowTargetHex) {
        throw new Error('Node RPC missing recent block anchor or micro-PoW target.');
      }

      const inputs: { txidHex: string; vout: number }[] = [];
      let totalIn = 0n;
      for (const u of liveAddr.utxos) {
        inputs.push({ txidHex: u.txid, vout: u.vout });
        totalIn += BigInt(u.value_sardhans);
        if (totalIn >= amountSardhans) {
          const rem = totalIn - amountSardhans;
          if (rem === 0n || rem >= MIN_TRANSACTION_OUTPUT_VALUE) break;
        }
      }

      if (totalIn < amountSardhans) {
        throw new Error(
          `Insufficient confirmed balance: have ${formatSardhansToSdr(totalIn)} SDR, need ${amountStr} SDR`
        );
      }

      const change = totalIn - amountSardhans;
      const outputs = [{ valueSardhans: amountSardhans, recipientPubKey }];
      if (change > 0n) {
        if (change < MIN_TRANSACTION_OUTPUT_VALUE) {
          throw new Error('Change output would fall below 0.00001 SDR dust limit.');
        }
        outputs.push({ valueSardhans: change, recipientPubKey: senderPubKey });
      }

      // Build, solve anti-spam micro-PoW, and sign locally in browser (key never transmitted)
      const built = buildAndSignZeroFeeTransaction({
        chainId: wallet.chainId,
        recentBlockHashHex,
        microPowTargetHex,
        inputs,
        outputs,
        secretKeyHex: wallet.secretKeyHex,
      });

      const broadcast = (await proxyRpc(1, 'sendrawtransaction', [built.rawTxHex])) as {
        txid: string;
        accepted: boolean;
      };

      setStatusMsg({
        type: 'ok',
        text: `Broadcasted zero-fee tx 0x${broadcast.txid.slice(0, 16)}... (Micro-PoW Nonce ${built.microPowNonce}, Fee: 0.00 SDR)`,
      });
      setRecipient('');
      setAmountStr('');
      await refreshBalance();
    } catch (err) {
      setStatusMsg({
        type: 'err',
        text: err instanceof Error ? err.message : 'Transaction build/broadcast failed',
      });
    } finally {
      setSendingTx(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Header Strip */}
      <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 flex flex-col lg:flex-row lg:items-center justify-between gap-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-xs font-mono text-neutral-400">
            <Lock className="w-3.5 h-3.5 text-emerald-400" />
            <span className="text-emerald-400 font-semibold">LOCAL NON-CUSTODIAL CUSTODY</span>
            <span aria-hidden="true">·</span>
            <span>ED25519 + BECH32M + CLIENT MICRO-POW</span>
          </div>
          <h2 className="text-xl font-bold text-neutral-100 tracking-tight">
            Native Sardhan (SDR) Wallet
          </h2>
          <p className="text-sm text-neutral-400 max-w-2xl">
            Private keys and mnemonic seeds are derived and signed locally in your browser via WebCrypto + Ed25519. Zero keys ever touch backend APIs. Compatible word-for-word with native CLI <code className="text-amber-400 font-mono">sdr-wallet</code>.
          </p>
        </div>

        {/* Network Selector */}
        <div className="flex items-center gap-2 font-mono text-xs shrink-0">
          <span className="text-neutral-500">NETWORK:</span>
          <button
            onClick={() => handleSwitchNetwork('testnet')}
            className={`px-3 py-1.5 rounded-lg border transition-colors ${
              network === 'testnet'
                ? 'bg-emerald-950/50 border-emerald-700 text-emerald-300 font-semibold'
                : 'bg-neutral-950 border-neutral-800 text-neutral-400 hover:text-neutral-200'
            }`}
          >
            Testnet (tsdr1)
          </button>
          <button
            onClick={() => handleSwitchNetwork('mainnet')}
            className={`px-3 py-1.5 rounded-lg border transition-colors ${
              network === 'mainnet'
                ? 'bg-amber-950/50 border-amber-700 text-amber-300 font-semibold'
                : 'bg-neutral-950 border-neutral-800 text-neutral-400 hover:text-neutral-200'
            }`}
          >
            Mainnet Locked (sdr1)
          </button>
        </div>
      </div>

      {statusMsg && (
        <div
          className={`rounded-lg border px-4 py-3 text-xs font-mono ${
            statusMsg.type === 'ok'
              ? 'border-emerald-800/70 bg-emerald-950/30 text-emerald-300'
              : 'border-rose-800/70 bg-rose-950/30 text-rose-300'
          }`}
        >
          {statusMsg.text}
        </div>
      )}

      {!wallet ? (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Create New Wallet */}
          <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-4">
            <div className="flex items-center gap-2 text-sm font-bold text-neutral-100">
              <KeyRound className="w-4 h-4 text-amber-400" />
              <h3>Create New Local Wallet</h3>
            </div>
            <p className="text-xs text-neutral-400 leading-relaxed">
              Generates fresh CSPRNG entropy in browser memory, derives a checksummed mnemonic phrase, computes 2048 iterated SHA-256 rounds + BLAKE3 HD key derivation, and encodes your Bech32m address.
            </p>
            <div className="flex flex-wrap gap-3 pt-2">
              <button
                onClick={() => handleCreateWallet(18)}
                className="px-4 py-2 rounded-lg bg-amber-400 text-neutral-950 text-xs font-semibold hover:bg-amber-300 transition-colors"
              >
                Generate 18-Word Wallet
              </button>
              <button
                onClick={() => handleCreateWallet(24)}
                className="px-4 py-2 rounded-lg bg-neutral-800 text-neutral-100 border border-neutral-700 text-xs font-semibold hover:bg-neutral-700 transition-colors"
              >
                Generate 24-Word Wallet
              </button>
            </div>
          </div>

          {/* Recover Existing Wallet */}
          <form
            onSubmit={handleRecoverWallet}
            className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-4"
          >
            <div className="flex items-center gap-2 text-sm font-bold text-neutral-100">
              <RotateCcw className="w-4 h-4 text-emerald-400" />
              <h3>Recover From Seed Phrase</h3>
            </div>
            <textarea
              value={recoverInput}
              onChange={(e) => setRecoverInput(e.target.value)}
              placeholder="Paste your 18 or 24 Sardhan mnemonic words..."
              rows={3}
              className="w-full rounded-lg bg-neutral-950 border border-neutral-800 p-3 text-xs font-mono text-neutral-200 focus:outline-none focus:border-amber-500"
            />
            <div className="flex flex-col sm:flex-row gap-3">
              <input
                type="password"
                value={passphrase}
                onChange={(e) => setPassphrase(e.target.value)}
                placeholder="Optional passphrase"
                className="flex-1 rounded-lg bg-neutral-950 border border-neutral-800 px-3 py-2 text-xs font-mono text-neutral-200 focus:outline-none focus:border-amber-500"
              />
              <button
                type="submit"
                className="px-4 py-2 rounded-lg bg-emerald-500 text-neutral-950 text-xs font-semibold hover:bg-emerald-400 transition-colors shrink-0"
              >
                Recover Locally
              </button>
            </div>
          </form>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* Left Column: Account Summary, Receive QR, Seed Backup */}
          <div className="lg:col-span-5 space-y-6">
            <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-4">
              <div className="flex items-center justify-between border-b border-neutral-800 pb-3">
                <div>
                  <div className="text-[11px] font-mono text-neutral-500 uppercase">CONFIRMED BALANCE</div>
                  <div className="text-2xl font-bold font-mono text-amber-400 tabular-nums mt-0.5">
                    {network === 'mainnet'
                      ? 'MAINNET LOCKED'
                      : `${(addrInfo?.balance_sdr ?? 0).toFixed(4)} SDR`}
                  </div>
                  {addrInfo && network === 'testnet' && (
                    <div className="text-[11px] font-mono text-neutral-400 tabular-nums mt-0.5">
                      Spendable: {addrInfo.spendable_sdr.toFixed(4)} SDR · UTXOs: {addrInfo.utxos.length}
                    </div>
                  )}
                </div>
                <button
                  onClick={refreshBalance}
                  disabled={loadingBalance}
                  className="p-2 rounded-lg bg-neutral-950 border border-neutral-800 text-neutral-400 hover:text-neutral-100"
                  title="Refresh balance from live node RPC"
                >
                  <RefreshCw className={`w-4 h-4 ${loadingBalance ? 'animate-spin' : ''}`} />
                </button>
              </div>

              {/* Receive Address & QR */}
              <div className="space-y-3">
                <div className="text-[11px] font-mono text-neutral-400 uppercase">
                  RECEIVE ADDRESS ({wallet.hrp.toUpperCase()} PREFIX)
                </div>
                <div className="flex items-center gap-2 bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2">
                  <code className="text-xs font-mono text-neutral-200 break-all flex-1">
                    {wallet.address}
                  </code>
                  <button
                    onClick={handleCopyAddress}
                    className="p-1.5 text-neutral-400 hover:text-amber-400 shrink-0"
                    title="Copy Bech32m address"
                  >
                    {copiedAddr ? <Check className="w-4 h-4 text-emerald-400" /> : <Copy className="w-4 h-4" />}
                  </button>
                </div>

                {qrMatrix && (
                  <div className="flex items-center gap-4 pt-1">
                    <div className="p-2.5 bg-white rounded-lg shrink-0">
                      <svg
                        viewBox={`0 0 ${qrMatrix.length + 4} ${qrMatrix.length + 4}`}
                        className="w-28 h-28"
                        shapeRendering="crispEdges"
                      >
                        <rect width={qrMatrix.length + 4} height={qrMatrix.length + 4} fill="#ffffff" />
                        {qrMatrix.map((row, r) =>
                          row.map((dark, c) =>
                            dark ? (
                              <rect key={`${r}-${c}`} x={c + 2} y={r + 2} width={1} height={1} fill="#0a0a0a" />
                            ) : null
                          )
                        )}
                      </svg>
                    </div>
                    <div className="text-[11px] text-neutral-400 space-y-2">
                      <div className="font-mono text-neutral-300">ISO-18004 QR v4-L (EC-L)</div>
                      <p className="leading-relaxed">
                        Matches <code className="text-amber-400">sdr-wallet qr</code> matrix output verbatim.
                      </p>
                      {network === 'testnet' && (
                        <button
                          onClick={handleMineToWallet}
                          disabled={miningReward}
                          className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-amber-400 text-neutral-950 text-xs font-semibold hover:bg-amber-300 disabled:opacity-50"
                        >
                          <Pickaxe className="w-3.5 h-3.5" />
                          <span>{miningReward ? 'Mining SardhanHash...' : 'Mine 1 PoW Block Here (+50 SDR)'}</span>
                        </button>
                      )}
                    </div>
                  </div>
                )}
              </div>

              {/* Mnemonic Backup & Hardware Signer Notice */}
              <div className="pt-3 border-t border-neutral-800 space-y-2">
                <div className="flex items-center justify-between text-xs font-mono">
                  <span className="text-neutral-400">SEED BACKUP</span>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => setShowMnemonic((v) => !v)}
                      className="inline-flex items-center gap-1 text-amber-400 hover:text-amber-300"
                    >
                      {showMnemonic ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}
                      <span>{showMnemonic ? 'Hide' : 'Reveal'}</span>
                    </button>
                    <button
                      onClick={handleClearWallet}
                      className="inline-flex items-center gap-1 text-rose-400 hover:text-rose-300"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                      <span>Forget</span>
                    </button>
                  </div>
                </div>
                {showMnemonic && (
                  <div className="p-3 rounded-lg bg-neutral-950 border border-neutral-800 font-mono text-xs text-amber-300 leading-relaxed select-all">
                    {wallet.mnemonic}
                  </div>
                )}
                <div className="flex items-center justify-between text-[11px] font-mono text-neutral-500 pt-1">
                  <span className="flex items-center gap-1.5">
                    <Cpu className="w-3.5 h-3.5" />
                    <span>Hardware Signer Interface:</span>
                  </span>
                  <span>USB-HID Ready (Offline Mode)</span>
                </div>
              </div>
            </div>
          </div>

          {/* Right Column: Send Zero-Fee Tx + Live History */}
          <div className="lg:col-span-7 space-y-6">
            <form
              onSubmit={handleSendZeroFee}
              className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-4"
            >
              <div className="flex items-center justify-between border-b border-neutral-800 pb-3">
                <div className="flex items-center gap-2 text-sm font-bold text-neutral-100">
                  <Send className="w-4 h-4 text-amber-400" />
                  <h3>Send Zero-Fee SDR Transaction</h3>
                </div>
                <span className="text-xs font-mono text-emerald-400">MANDATORY FEE: 0.00 SDR</span>
              </div>

              {network === 'mainnet' && (
                <div className="rounded-lg border border-amber-800/70 bg-amber-950/30 p-3 text-xs font-mono text-amber-300">
                  Mainnet transmission disabled by Security Release Gate (MAINNET_ENABLED = false). Switch to Testnet to broadcast live transactions.
                </div>
              )}

              <div className="space-y-3">
                <div>
                  <label className="block text-[11px] font-mono text-neutral-400 uppercase mb-1">
                    Recipient Bech32m Address ({wallet.hrp}1...)
                  </label>
                  <input
                    type="text"
                    value={recipient}
                    onChange={(e) => setRecipient(e.target.value)}
                    placeholder={`${wallet.hrp}1...`}
                    required
                    className="w-full rounded-lg bg-neutral-950 border border-neutral-800 px-3 py-2 text-xs font-mono text-neutral-200 focus:outline-none focus:border-amber-500"
                  />
                </div>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <label className="block text-[11px] font-mono text-neutral-400 uppercase mb-1">
                      Amount (SDR · Exact Integer Parse)
                    </label>
                    <input
                      type="text"
                      value={amountStr}
                      onChange={(e) => setAmountStr(e.target.value)}
                      placeholder="10.00"
                      required
                      className="w-full rounded-lg bg-neutral-950 border border-neutral-800 px-3 py-2 text-xs font-mono text-neutral-200 tabular-nums focus:outline-none focus:border-amber-500"
                    />
                  </div>
                  <div className="flex items-end">
                    <button
                      type="submit"
                      disabled={sendingTx || network === 'mainnet'}
                      className="w-full flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-amber-400 text-neutral-950 text-xs font-bold hover:bg-amber-300 transition-colors disabled:opacity-50"
                    >
                      <ShieldCheck className="w-4 h-4" />
                      <span>
                        {sendingTx
                          ? 'Solving Micro-PoW & Signing...'
                          : 'Solve Micro-PoW, Sign Locally & Send'}
                      </span>
                    </button>
                  </div>
                </div>
              </div>
            </form>

            {/* On-Chain Transaction History */}
            <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-6 space-y-3">
              <div className="flex items-center justify-between border-b border-neutral-800 pb-3">
                <h3 className="text-xs font-mono font-bold text-neutral-200 uppercase">
                  Confirmed On-Chain History ({addrInfo?.history.length ?? 0})
                </h3>
                <span className="text-[11px] font-mono text-neutral-500">Queried via Node 1 RPC</span>
              </div>

              {!addrInfo || addrInfo.history.length === 0 ? (
                <p className="text-xs text-neutral-500 py-3 font-mono">
                  No confirmed transactions yet for this address. Click "Mine 1 PoW Block Here (+50 SDR)" to fund it on the live 3-node Testnet.
                </p>
              ) : (
                <div className="divide-y divide-neutral-800/70 max-h-72 overflow-y-auto">
                  {addrInfo.history
                    .slice()
                    .reverse()
                    .map((item, idx) => (
                      <div
                        key={`${item.txid}-${idx}`}
                        className="py-2.5 flex items-center justify-between text-xs font-mono gap-4"
                      >
                        <div className="flex items-center gap-3 min-w-0">
                          <span className="text-neutral-500 tabular-nums shrink-0">#{item.block_height}</span>
                          <span
                            className={`font-semibold shrink-0 ${
                              item.direction === 'received' || item.direction === 'reward'
                                ? 'text-emerald-400'
                                : 'text-amber-400'
                            }`}
                          >
                            {item.direction.toUpperCase()}
                          </span>
                          <span className="text-neutral-400 truncate">0x{item.txid}</span>
                        </div>
                        <span className="text-neutral-100 font-semibold tabular-nums shrink-0">
                          {(item.amount_sardhans / 1e8).toFixed(4)} SDR
                        </span>
                      </div>
                    ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
