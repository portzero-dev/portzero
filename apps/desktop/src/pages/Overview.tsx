import { useQuery } from "@tanstack/react-query";
import { listApps, getDaemonStatus, getDaemonInfo } from "../api/client";
import { AppCard } from "../components/AppCard";
import { LiveRequestFeed } from "../components/LiveRequestFeed";
import { formatUptime } from "../lib/formatters";
import type { Route } from "../App";
import {
  Activity,
  Server,
  Loader2,
  AlertCircle,
} from "lucide-react";

interface OverviewProps {
  navigate: (route: Route) => void;
}

export function Overview({ navigate }: OverviewProps) {
  const {
    data: apps,
    isLoading: appsLoading,
    error: appsError,
  } = useQuery({
    queryKey: ["apps"],
    queryFn: listApps,
    refetchInterval: 5000,
  });

  const { data: status } = useQuery({
    queryKey: ["status"],
    queryFn: getDaemonStatus,
  });

  const { data: daemonInfo } = useQuery({
    queryKey: ["daemon-info"],
    queryFn: getDaemonInfo,
    refetchInterval: 5_000,
  });

  const daemonRunning = daemonInfo?.running && daemonInfo?.responsive;

  return (
    <div className="p-6">
      {/* Header */}
      <div className="mb-6">
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-bold text-zinc-100">Overview</h1>
          {daemonInfo && (
            <span
              className={`flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium ${
                daemonRunning
                  ? "bg-emerald-900/40 text-emerald-400"
                  : "bg-red-900/40 text-red-400"
              }`}
              title={
                daemonRunning
                  ? `Daemon running (PID ${daemonInfo.pid})`
                  : "Daemon not running"
              }
            >
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  daemonRunning ? "bg-emerald-400" : "bg-red-400"
                }`}
              />
              {daemonRunning ? "Daemon Running" : "Daemon Stopped"}
            </span>
          )}
        </div>
        {status && (
          <p className="mt-1 text-sm text-zinc-500">
            Daemon v{status.version} - Up {formatUptime(status.uptime_seconds)}{" "}
            - {status.total_requests.toLocaleString()} total requests
          </p>
        )}
      </div>

      {/* Stats cards */}
      {status && (
        <div className="mb-6 grid grid-cols-3 gap-4">
          <StatCard
            label="Apps"
            value={status.app_count}
            icon={<Server size={16} className="text-violet-400" />}
          />
          <StatCard
            label="Total Requests"
            value={status.total_requests.toLocaleString()}
            icon={<Activity size={16} className="text-blue-400" />}
          />
          <StatCard
            label="Proxy Port"
            value={status.proxy_port}
            icon={<Server size={16} className="text-emerald-400" />}
          />
        </div>
      )}

      {/* Apps grid */}
      <section className="mb-8">
        <h2 className="mb-3 text-sm font-semibold text-zinc-300">Apps</h2>
        {appsLoading ? (
          <div className="flex items-center justify-center py-12 text-zinc-500">
            <Loader2 size={20} className="animate-spin" />
          </div>
        ) : appsError ? (
          <div className="flex items-center gap-2 rounded-xl border border-red-800/50 bg-red-900/10 px-4 py-3 text-sm text-red-400">
            <AlertCircle size={16} />
            Failed to load apps. Is the daemon running?
          </div>
        ) : apps && apps.length > 0 ? (
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2 xl:grid-cols-3">
            {apps.map((app) => (
              <AppCard key={app.name} app={app} navigate={navigate} />
            ))}
          </div>
        ) : (
          <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-8 text-center text-sm text-zinc-500">
            No apps running. Start one with{" "}
            <code className="rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-violet-400">
              portzero my-app next dev
            </code>
          </div>
        )}
      </section>

      {/* Live request feed */}
      <section>
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold text-zinc-300">
            Live Traffic
          </h2>
          <button
            type="button"
            onClick={() => navigate({ page: "traffic" })}
            className="text-xs text-violet-400 hover:text-violet-300"
          >
            View all traffic
          </button>
        </div>
        <LiveRequestFeed navigate={navigate} />
      </section>
    </div>
  );
}

function StatCard({
  label,
  value,
  icon,
}: {
  label: string;
  value: string | number;
  icon: React.ReactNode;
}) {
  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
      <div className="mb-1 flex items-center gap-2 text-xs text-zinc-500">
        {icon}
        {label}
      </div>
      <div className="text-2xl font-bold text-zinc-100">{value}</div>
    </div>
  );
}
