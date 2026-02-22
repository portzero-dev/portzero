import { useQuery } from "@tanstack/react-query";
import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getApp, listRequests } from "../api/client";
import { StatusBadge } from "../components/StatusBadge";
import { LogViewer } from "../components/LogViewer";
import { SchemaViewer } from "../components/SchemaViewer";
import { TunnelStatus } from "../components/TunnelStatus";
import { NetworkSimPanel } from "../components/NetworkSimPanel";
import { RequestRow } from "../components/RequestRow";
import {
  formatUptime,
  formatBytes,
  formatCpu,
} from "../lib/formatters";
import type { Route } from "../App";
import type { WsEvent } from "../lib/types";
import {
  ArrowLeft,
  Loader2,
  ExternalLink,
  Terminal,
  XCircle,
  ChevronDown,
} from "lucide-react";

interface AppDetailPageProps {
  name: string;
  navigate: (route: Route) => void;
}

export function AppDetailPage({ name, navigate }: AppDetailPageProps) {
  const [removed, setRemoved] = useState(false);
  const countdownRef = useRef<number | null>(null);
  const [countdown, setCountdown] = useState(5);
  const [logsOpen, setLogsOpen] = useState(false);
  const [schemaOpen, setSchemaOpen] = useState(false);

  const { data: app, isLoading, error } = useQuery({
    queryKey: ["app", name],
    queryFn: () => getApp(name),
    refetchInterval: removed ? false : 5000,
    retry: (failureCount, err) => {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes("not found")) return false;
      return failureCount < 2;
    },
  });

  // Detect removal from query error
  useEffect(() => {
    if (error && !removed) {
      const msg = error instanceof Error ? error.message : String(error);
      if (msg.includes("not found")) {
        setRemoved(true);
      }
    }
  }, [error, removed]);

  // Listen for app:removed ws-event
  useEffect(() => {
    const unlisten = listen<WsEvent>("ws-event", (tauriEvent) => {
      const event = tauriEvent.payload;
      if (event.type === "app:removed" && event.name === name) {
        setRemoved(true);
      }
      if (event.type === "app:crashed" && event.name === name) {
        setRemoved(true);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [name]);

  // Auto-navigate back after countdown when removed
  useEffect(() => {
    if (!removed) return;

    setCountdown(5);
    const interval = window.setInterval(() => {
      setCountdown((prev) => {
        if (prev <= 1) {
          navigate({ page: "overview" });
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    countdownRef.current = interval;

    return () => {
      if (countdownRef.current) window.clearInterval(countdownRef.current);
    };
  }, [removed, navigate]);

  const { data: appRequests } = useQuery({
    queryKey: ["requests", { app: name, limit: 50 }],
    queryFn: () => listRequests({ app: name, limit: 50 }),
    enabled: !removed,
    refetchInterval: 5000,
  });

  // App was removed
  if (removed) {
    return (
      <div className="p-6">
        <button
          type="button"
          onClick={() => navigate({ page: "overview" })}
          className="mb-4 flex items-center gap-1.5 text-sm text-zinc-400 hover:text-zinc-200"
        >
          <ArrowLeft size={14} />
          Back
        </button>
        <div className="mx-auto mt-16 flex max-w-md flex-col items-center text-center">
          <div className="mb-4 rounded-full bg-zinc-800 p-4">
            <XCircle size={32} className="text-zinc-500" />
          </div>
          <h2 className="text-lg font-semibold text-zinc-200">App removed</h2>
          <p className="mt-2 text-sm text-zinc-500">
            <span className="font-mono text-zinc-400">{name}</span> is no
            longer running. The process was stopped or the{" "}
            <span className="font-mono text-zinc-400">portzero</span> command
            was killed.
          </p>
          <p className="mt-4 text-xs text-zinc-600">
            Returning to overview in {countdown}s...
          </p>
          <button
            type="button"
            onClick={() => navigate({ page: "overview" })}
            className="mt-4 rounded-lg bg-violet-600 px-4 py-2 text-sm font-medium text-white hover:bg-violet-700"
          >
            Go to Overview
          </button>
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-24 text-zinc-500">
        <Loader2 size={24} className="animate-spin" />
      </div>
    );
  }

  if (error || !app) {
    return (
      <div className="p-6">
        <button
          type="button"
          onClick={() => navigate({ page: "overview" })}
          className="mb-4 flex items-center gap-1.5 text-sm text-zinc-400 hover:text-zinc-200"
        >
          <ArrowLeft size={14} />
          Back
        </button>
        <div className="mx-auto mt-16 flex max-w-md flex-col items-center text-center">
          <div className="mb-4 rounded-full bg-zinc-800 p-4">
            <XCircle size={32} className="text-zinc-500" />
          </div>
          <h2 className="text-lg font-semibold text-zinc-200">App not found</h2>
          <p className="mt-2 text-sm text-zinc-500">
            <span className="font-mono text-zinc-400">{name}</span> could not be
            loaded. It may have been removed or the daemon may not be reachable.
          </p>
          <button
            type="button"
            onClick={() => navigate({ page: "overview" })}
            className="mt-4 rounded-lg bg-violet-600 px-4 py-2 text-sm font-medium text-white hover:bg-violet-700"
          >
            Go to Overview
          </button>
        </div>
      </div>
    );
  }

  const uptimeSeconds =
    app.status.type === "running" && app.started_at
      ? Math.floor((Date.now() - new Date(app.started_at).getTime()) / 1000)
      : 0;

  return (
    <div className="p-6">
      {/* Back button */}
      <button
        type="button"
        onClick={() => navigate({ page: "overview" })}
        className="mb-4 flex items-center gap-1.5 text-sm text-zinc-400 hover:text-zinc-200"
      >
        <ArrowLeft size={14} />
        Back
      </button>

      {/* Header: name + status + url + stats */}
      <div className="mb-4 flex items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex items-center gap-3">
            <h1 className="text-xl font-bold text-zinc-100">{app.name}</h1>
            <StatusBadge status={app.status} />
          </div>
          <div className="mt-1 flex items-center gap-3">
            <a
              href={app.url}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-sm text-violet-400 hover:text-violet-300"
            >
              {app.url}
              <ExternalLink size={12} />
            </a>
            {app.command.length > 0 && (
              <div className="flex items-center gap-1.5 rounded bg-zinc-800/60 px-2 py-0.5">
                <Terminal size={12} className="text-zinc-500" />
                <code className="text-xs text-zinc-500">
                  {app.command.join(" ")}
                </code>
              </div>
            )}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2 text-xs">
          <StatPill label="Port" value={String(app.port)} />
          <StatPill
            label="PID"
            value={
              app.status.type === "running" && app.pid != null
                ? String(app.pid)
                : "-"
            }
          />
          <StatPill
            label="Uptime"
            value={
              app.status.type === "running" ? formatUptime(uptimeSeconds) : "-"
            }
          />
          {app.cpu_usage != null && (
            <StatPill label="CPU" value={formatCpu(app.cpu_usage)} />
          )}
          {app.memory_bytes != null && (
            <StatPill label="Mem" value={formatBytes(app.memory_bytes)} />
          )}
        </div>
      </div>

      {/* Traffic — primary content, right at the top */}
      <div className="mb-6">
        <div className="mb-2 flex items-center justify-between">
          <h2 className="text-sm font-semibold text-zinc-300">Traffic</h2>
          <button
            type="button"
            onClick={() => navigate({ page: "traffic" })}
            className="text-xs text-violet-400 hover:text-violet-300"
          >
            View all traffic
          </button>
        </div>
        <div className="max-h-[420px] overflow-auto rounded-xl border border-zinc-800 bg-zinc-900">
          {appRequests && appRequests.length > 0 ? (
            appRequests.map((req) => (
              <RequestRow key={req.id} request={req} navigate={navigate} />
            ))
          ) : (
            <div className="p-6 text-center text-sm text-zinc-500">
              No traffic captured for this app
            </div>
          )}
        </div>
      </div>

      {/* Network + Tunnel — side by side */}
      <div className="mb-6 grid grid-cols-2 gap-4">
        <NetworkSimPanel appName={app.name} />
        <TunnelStatus />
      </div>

      {/* Logs — collapsible */}
      <div className="mb-4">
        <button
          type="button"
          onClick={() => setLogsOpen(!logsOpen)}
          className="mb-2 flex items-center gap-1.5 text-sm font-semibold text-zinc-300 hover:text-zinc-100"
        >
          <ChevronDown
            size={14}
            className={`transition-transform ${logsOpen ? "" : "-rotate-90"}`}
          />
          Logs
        </button>
        {logsOpen && <LogViewer appName={app.name} />}
      </div>

      {/* Schema — collapsible */}
      <div>
        <button
          type="button"
          onClick={() => setSchemaOpen(!schemaOpen)}
          className="mb-2 flex items-center gap-1.5 text-sm font-semibold text-zinc-300 hover:text-zinc-100"
        >
          <ChevronDown
            size={14}
            className={`transition-transform ${schemaOpen ? "" : "-rotate-90"}`}
          />
          API Schema
        </button>
        {schemaOpen && <SchemaViewer appName={app.name} />}
      </div>
    </div>
  );
}

function StatPill({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900 px-2.5 py-1.5 text-center">
      <div className="text-[10px] uppercase tracking-wider text-zinc-500">
        {label}
      </div>
      <div className="font-mono text-sm font-medium text-zinc-200">
        {value}
      </div>
    </div>
  );
}
