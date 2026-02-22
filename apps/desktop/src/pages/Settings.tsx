import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  getCertStatus,
  trustCA,
  untrustCA,
  getCliStatus,
  installCli,
  uninstallCli,
  getDaemonInfo,
  startDaemon,
  stopDaemon,
  restartDaemon,
} from "../api/client";
import {
  ShieldCheck,
  ShieldAlert,
  ShieldOff,
  Loader2,
  Lock,
  Unlock,
  Copy,
  Check,
  AlertCircle,
  Terminal,
  Download,
  Trash2,
  CheckCircle2,
  XCircle,
  FolderSymlink,
  Server,
  Play,
  Square,
  RotateCw,
} from "lucide-react";
import { useState } from "react";

export function Settings() {
  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-xl font-bold text-zinc-100">Settings</h1>
        <p className="mt-1 text-sm text-zinc-500">
          Manage PortZero configuration and HTTPS certificates
        </p>
      </div>

      <div className="space-y-6">
        <DaemonSection />
        <CliInstallSection />
        <CertificateSection />
      </div>
    </div>
  );
}

function DaemonSection() {
  const queryClient = useQueryClient();

  const {
    data: daemonInfo,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["daemon-info"],
    queryFn: getDaemonInfo,
    refetchInterval: 5_000,
  });

  const startMutation = useMutation({
    mutationFn: startDaemon,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["daemon-info"] });
      queryClient.invalidateQueries({ queryKey: ["status"] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    },
  });

  const stopMutation = useMutation({
    mutationFn: stopDaemon,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["daemon-info"] });
      queryClient.invalidateQueries({ queryKey: ["status"] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    },
  });

  const restartMutation = useMutation({
    mutationFn: restartDaemon,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["daemon-info"] });
      queryClient.invalidateQueries({ queryKey: ["status"] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    },
  });

  const isRunning = daemonInfo?.running && daemonInfo?.responsive;
  const isStale = daemonInfo?.running && !daemonInfo?.responsive;
  const isBusy =
    startMutation.isPending ||
    stopMutation.isPending ||
    restartMutation.isPending;

  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-zinc-800 px-5 py-4">
        <div className="flex items-center gap-3">
          <Server size={20} className="text-violet-400" />
          <div>
            <h2 className="text-base font-semibold text-zinc-100">
              Proxy Daemon
            </h2>
            <p className="text-sm text-zinc-500">
              The Pingora reverse proxy that routes *.localhost traffic
            </p>
          </div>
        </div>

        {/* Status badge */}
        {!isLoading && !error && (
          <div
            className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium ${
              isRunning
                ? "bg-emerald-900/40 text-emerald-400"
                : isStale
                  ? "bg-amber-900/40 text-amber-400"
                  : "bg-zinc-800 text-zinc-400"
            }`}
          >
            {isRunning ? (
              <>
                <span className="relative flex h-2 w-2">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75" />
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-500" />
                </span>
                Running
              </>
            ) : isStale ? (
              <>
                <AlertCircle size={12} />
                Not Responding
              </>
            ) : (
              <>
                <XCircle size={12} />
                Stopped
              </>
            )}
          </div>
        )}
      </div>

      {/* Body */}
      <div className="space-y-4 px-5 py-4">
        {isLoading && (
          <div className="flex items-center gap-2 text-sm text-zinc-400">
            <Loader2 size={14} className="animate-spin" />
            Checking daemon status...
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-red-800/50 bg-red-950/30 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-red-400">
              <AlertCircle size={14} />
              Failed to check daemon status
            </div>
          </div>
        )}

        {/* PID info */}
        {daemonInfo?.pid && (
          <div className="flex items-center gap-2 text-sm text-zinc-400">
            <span className="font-medium text-zinc-300">PID:</span>
            <code className="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400">
              {daemonInfo.pid}
            </code>
          </div>
        )}

        {/* Stale daemon warning */}
        {isStale && (
          <div className="rounded-lg border border-amber-800/30 bg-amber-950/20 px-4 py-3">
            <p className="text-sm text-amber-300/80">
              The daemon process is running (PID {daemonInfo?.pid}) but not
              responding on the control socket. It may have crashed or be in a
              bad state. Try restarting it.
            </p>
          </div>
        )}

        {/* Not running info */}
        {!isLoading && !error && !daemonInfo?.running && (
          <div className="rounded-lg border border-blue-800/30 bg-blue-950/20 px-4 py-3">
            <p className="text-sm text-blue-300/80">
              The proxy daemon is not running. Start it to route traffic through{" "}
              <code className="text-blue-200">*.localhost:1337</code> to your
              dev servers.
            </p>
          </div>
        )}

        {/* Error messages from mutations */}
        {startMutation.error && (
          <div className="rounded-lg border border-red-800/50 bg-red-950/30 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-red-400">
              <AlertCircle size={14} />
              {String(
                startMutation.error instanceof Error
                  ? startMutation.error.message
                  : startMutation.error,
              )}
            </div>
          </div>
        )}
        {stopMutation.error && (
          <div className="rounded-lg border border-red-800/50 bg-red-950/30 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-red-400">
              <AlertCircle size={14} />
              {String(
                stopMutation.error instanceof Error
                  ? stopMutation.error.message
                  : stopMutation.error,
              )}
            </div>
          </div>
        )}
        {restartMutation.error && (
          <div className="rounded-lg border border-red-800/50 bg-red-950/30 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-red-400">
              <AlertCircle size={14} />
              {String(
                restartMutation.error instanceof Error
                  ? restartMutation.error.message
                  : restartMutation.error,
              )}
            </div>
          </div>
        )}

        {/* Action buttons */}
        <div className="flex items-center gap-3">
          {!isRunning && (
            <button
              type="button"
              onClick={() => startMutation.mutate()}
              disabled={isBusy}
              className="flex items-center gap-2 rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-emerald-500 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {startMutation.isPending ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <Play size={14} />
              )}
              {startMutation.isPending ? "Starting..." : "Start Daemon"}
            </button>
          )}

          {isRunning && (
            <>
              <button
                type="button"
                onClick={() => restartMutation.mutate()}
                disabled={isBusy}
                className="flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {restartMutation.isPending ? (
                  <Loader2 size={14} className="animate-spin" />
                ) : (
                  <RotateCw size={14} />
                )}
                {restartMutation.isPending ? "Restarting..." : "Restart"}
              </button>

              <button
                type="button"
                onClick={() => stopMutation.mutate()}
                disabled={isBusy}
                className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-300 transition-colors hover:bg-zinc-700 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {stopMutation.isPending ? (
                  <Loader2 size={14} className="animate-spin" />
                ) : (
                  <Square size={14} />
                )}
                {stopMutation.isPending ? "Stopping..." : "Stop"}
              </button>
            </>
          )}

          {isStale && (
            <button
              type="button"
              onClick={() => restartMutation.mutate()}
              disabled={isBusy}
              className="flex items-center gap-2 rounded-lg bg-amber-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-amber-500 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {restartMutation.isPending ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <RotateCw size={14} />
              )}
              {restartMutation.isPending ? "Restarting..." : "Force Restart"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function CliInstallSection() {
  const queryClient = useQueryClient();
  const [lastResult, setLastResult] = useState<string | null>(null);

  const {
    data: cliStatus,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["cli-status"],
    queryFn: getCliStatus,
    refetchInterval: 10_000,
  });

  const installMutation = useMutation({
    mutationFn: () => installCli(),
    onSuccess: (result) => {
      setLastResult(result.message);
      queryClient.invalidateQueries({ queryKey: ["cli-status"] });
    },
  });

  const uninstallMutation = useMutation({
    mutationFn: uninstallCli,
    onSuccess: (result) => {
      setLastResult(result.message);
      queryClient.invalidateQueries({ queryKey: ["cli-status"] });
    },
  });

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 text-zinc-400">
        <Loader2 size={16} className="animate-spin" />
        Checking CLI status...
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-lg border border-red-800/50 bg-red-950/30 p-4">
        <div className="flex items-center gap-2 text-red-400">
          <AlertCircle size={16} />
          <span className="text-sm">Failed to check CLI status.</span>
        </div>
      </div>
    );
  }

  const installed = cliStatus?.installed;
  const binaryExists = cliStatus?.binary_exists;

  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-zinc-800 px-5 py-4">
        <div className="flex items-center gap-3">
          <Terminal size={20} className="text-blue-400" />
          <div>
            <h2 className="text-base font-semibold text-zinc-100">
              CLI Tool
            </h2>
            <p className="text-sm text-zinc-500">
              Install the <code className="text-zinc-400">portzero</code> command in your PATH
            </p>
          </div>
        </div>

        {/* Status badge */}
        <div
          className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium ${
            installed
              ? "bg-emerald-900/40 text-emerald-400"
              : "bg-zinc-800 text-zinc-400"
          }`}
        >
          {installed ? (
            <>
              <CheckCircle2 size={12} />
              Installed
            </>
          ) : (
            <>
              <XCircle size={12} />
              Not Installed
            </>
          )}
        </div>
      </div>

      {/* Body */}
      <div className="space-y-4 px-5 py-4">
        {/* Current installation info */}
        {installed && cliStatus?.current_path && (
          <div className="flex items-center gap-2 text-sm text-zinc-400">
            <FolderSymlink size={14} className="text-zinc-500" />
            <span className="font-medium text-zinc-300">Location:</span>
            <code className="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400">
              {cliStatus.current_path}
            </code>
          </div>
        )}

        {/* Binary source info */}
        {binaryExists && cliStatus?.binary_path && (
          <div className="flex items-center gap-2 text-sm text-zinc-400">
            <Terminal size={14} className="text-zinc-500" />
            <span className="font-medium text-zinc-300">Binary:</span>
            <code className="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400 truncate max-w-md">
              {cliStatus.binary_path}
            </code>
          </div>
        )}

        {/* Not built warning */}
        {!binaryExists && (
          <div className="rounded-lg border border-amber-800/30 bg-amber-950/20 px-4 py-3">
            <p className="text-sm text-amber-300/80">
              The CLI binary hasn't been built yet. Run{" "}
              <code className="text-amber-200">
                cargo build -p portzero-cli
              </code>{" "}
              first, then come back to install it.
            </p>
          </div>
        )}

        {/* Explanation when not installed */}
        {!installed && binaryExists && (
          <div className="rounded-lg border border-blue-800/30 bg-blue-950/20 px-4 py-3">
            <p className="text-sm text-blue-300/80">
              Install the <code className="text-blue-200">portzero</code>{" "}
              command to manage dev servers from your terminal. It will be
              symlinked to{" "}
              <code className="text-blue-200">{cliStatus?.install_dir}/portzero</code>.
            </p>
          </div>
        )}

        {/* Error / success messages */}
        {installMutation.error && (
          <div className="rounded-lg border border-red-800/50 bg-red-950/30 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-red-400">
              <AlertCircle size={14} />
              {String(
                installMutation.error instanceof Error
                  ? installMutation.error.message
                  : installMutation.error,
              )}
            </div>
          </div>
        )}
        {uninstallMutation.error && (
          <div className="rounded-lg border border-red-800/50 bg-red-950/30 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-red-400">
              <AlertCircle size={14} />
              {String(
                uninstallMutation.error instanceof Error
                  ? uninstallMutation.error.message
                  : uninstallMutation.error,
              )}
            </div>
          </div>
        )}
        {lastResult && (
          <div className="rounded-lg border border-emerald-800/30 bg-emerald-950/20 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-emerald-400">
              <Check size={14} />
              {lastResult}
            </div>
          </div>
        )}

        {/* Action buttons */}
        <div className="flex items-center gap-3">
          {!installed && binaryExists && (
            <button
              type="button"
              onClick={() => {
                setLastResult(null);
                installMutation.mutate();
              }}
              disabled={installMutation.isPending}
              className="flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {installMutation.isPending ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <Download size={14} />
              )}
              {installMutation.isPending
                ? "Installing..."
                : "Install CLI to PATH"}
            </button>
          )}

          {installed && (
            <button
              type="button"
              onClick={() => {
                setLastResult(null);
                uninstallMutation.mutate();
              }}
              disabled={uninstallMutation.isPending}
              className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-300 transition-colors hover:bg-zinc-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {uninstallMutation.isPending ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <Trash2 size={14} />
              )}
              {uninstallMutation.isPending
                ? "Removing..."
                : "Uninstall CLI"}
            </button>
          )}
        </div>

        {/* Manual command hint */}
        {!installed && binaryExists && (
          <details className="group">
            <summary className="cursor-pointer text-xs text-zinc-500 hover:text-zinc-400">
              Manual alternative (terminal command)
            </summary>
            <div className="mt-2">
              <code className="block rounded bg-zinc-800 px-3 py-2 text-xs text-zinc-400 font-mono">
                sudo ln -sf {cliStatus?.binary_path} {cliStatus?.install_dir}/portzero
              </code>
            </div>
          </details>
        )}
      </div>
    </div>
  );
}

function CertificateSection() {
  const queryClient = useQueryClient();
  const [copied, setCopied] = useState(false);

  const {
    data: certStatus,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["cert-status"],
    queryFn: getCertStatus,
    refetchInterval: 10_000,
  });

  const trustMutation = useMutation({
    mutationFn: trustCA,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cert-status"] });
    },
  });

  const untrustMutation = useMutation({
    mutationFn: untrustCA,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cert-status"] });
    },
  });

  const handleCopyCommand = () => {
    if (certStatus?.trust_command) {
      navigator.clipboard.writeText(certStatus.trust_command);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 text-zinc-400">
        <Loader2 size={16} className="animate-spin" />
        Loading certificate status...
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-lg border border-red-800/50 bg-red-950/30 p-4">
        <div className="flex items-center gap-2 text-red-400">
          <AlertCircle size={16} />
          <span className="text-sm">
            Failed to load certificate status. Is the daemon running?
          </span>
        </div>
      </div>
    );
  }

  const isTrusted = certStatus?.ca_trusted;
  const certsExist = certStatus?.certs_exist;

  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-zinc-800 px-5 py-4">
        <div className="flex items-center gap-3">
          {isTrusted ? (
            <ShieldCheck size={20} className="text-emerald-400" />
          ) : (
            <ShieldAlert size={20} className="text-amber-400" />
          )}
          <div>
            <h2 className="text-base font-semibold text-zinc-100">
              HTTPS Certificates
            </h2>
            <p className="text-sm text-zinc-500">
              PortZero generates a local CA to serve *.localhost over HTTPS
            </p>
          </div>
        </div>

        {/* Status badge */}
        <div
          className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium ${
            isTrusted
              ? "bg-emerald-900/40 text-emerald-400"
              : "bg-amber-900/40 text-amber-400"
          }`}
        >
          {isTrusted ? (
            <>
              <Lock size={12} />
              Trusted
            </>
          ) : (
            <>
              <Unlock size={12} />
              Not Trusted
            </>
          )}
        </div>
      </div>

      {/* Body */}
      <div className="space-y-4 px-5 py-4">
        {/* Certificate info */}
        {certsExist && (
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-sm text-zinc-400">
              <span className="font-medium text-zinc-300">CA Certificate:</span>
              <code className="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400">
                {certStatus?.ca_cert_path}
              </code>
            </div>
          </div>
        )}

        {/* Trust explanation */}
        {!isTrusted && (
          <div className="rounded-lg border border-amber-800/30 bg-amber-950/20 px-4 py-3">
            <p className="text-sm text-amber-300/80">
              Your browser will show security warnings for{" "}
              <code className="text-amber-200">*.localhost</code> URLs until
              you trust the PortZero CA certificate. Click the button below
              to add it to your system trust store (requires your password).
            </p>
          </div>
        )}

        {/* Error messages */}
        {trustMutation.error && (
          <div className="rounded-lg border border-red-800/50 bg-red-950/30 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-red-400">
              <AlertCircle size={14} />
              {String(trustMutation.error instanceof Error ? trustMutation.error.message : trustMutation.error)}
            </div>
          </div>
        )}
        {untrustMutation.error && (
          <div className="rounded-lg border border-red-800/50 bg-red-950/30 px-4 py-3">
            <div className="flex items-center gap-2 text-sm text-red-400">
              <AlertCircle size={14} />
              {String(untrustMutation.error instanceof Error ? untrustMutation.error.message : untrustMutation.error)}
            </div>
          </div>
        )}

        {/* Action buttons */}
        <div className="flex items-center gap-3">
          {!isTrusted ? (
            <button
              type="button"
              onClick={() => trustMutation.mutate()}
              disabled={trustMutation.isPending}
              className="flex items-center gap-2 rounded-lg bg-violet-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-violet-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {trustMutation.isPending ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <ShieldCheck size={14} />
              )}
              {trustMutation.isPending
                ? "Waiting for password..."
                : "Trust CA Certificate"}
            </button>
          ) : (
            <button
              type="button"
              onClick={() => untrustMutation.mutate()}
              disabled={untrustMutation.isPending}
              className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-300 transition-colors hover:bg-zinc-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {untrustMutation.isPending ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <ShieldOff size={14} />
              )}
              {untrustMutation.isPending
                ? "Waiting for password..."
                : "Remove Trust"}
            </button>
          )}
        </div>

        {/* Manual command fallback */}
        {!isTrusted && certStatus?.trust_command && (
          <details className="group">
            <summary className="cursor-pointer text-xs text-zinc-500 hover:text-zinc-400">
              Manual alternative (terminal command)
            </summary>
            <div className="mt-2 flex items-center gap-2">
              <code className="flex-1 rounded bg-zinc-800 px-3 py-2 text-xs text-zinc-400 font-mono">
                {certStatus.trust_command}
              </code>
              <button
                type="button"
                onClick={handleCopyCommand}
                className="flex-shrink-0 rounded p-1.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300"
                title="Copy to clipboard"
              >
                {copied ? <Check size={14} /> : <Copy size={14} />}
              </button>
            </div>
          </details>
        )}
      </div>
    </div>
  );
}
