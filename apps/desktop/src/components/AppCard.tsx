import type { AppInfo } from "../lib/types";
import type { Route } from "../App";
import { StatusBadge } from "./StatusBadge";
import { formatUptime, formatBytes, formatCpu } from "../lib/formatters";
import {
  ExternalLink,
  Terminal,
} from "lucide-react";

interface AppCardProps {
  app: AppInfo;
  navigate: (route: Route) => void;
}

export function AppCard({ app, navigate }: AppCardProps) {
  const uptimeSeconds =
    app.status.type === "running" && app.started_at
      ? Math.floor((Date.now() - new Date(app.started_at).getTime()) / 1000)
      : 0;

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-4 transition-colors hover:border-zinc-700">
      {/* Header */}
      <div className="mb-3 flex items-start justify-between">
        <button
          type="button"
          onClick={() => navigate({ page: "app", name: app.name })}
          className="group flex items-center gap-2"
        >
          <h3 className="text-base font-semibold text-zinc-100 group-hover:text-violet-400 transition-colors">
            {app.name}
          </h3>
          <ExternalLink
            size={14}
            className="text-zinc-500 opacity-0 transition-opacity group-hover:opacity-100"
          />
        </button>
        <StatusBadge status={app.status} />
      </div>

      {/* URL */}
      <a
        href={app.url}
        target="_blank"
        rel="noopener noreferrer"
        className="mb-3 block truncate text-sm text-violet-400 hover:text-violet-300"
      >
        {app.url}
      </a>

      {/* Command */}
      <div className="mb-3 flex items-center gap-2 rounded-lg bg-zinc-800/50 px-3 py-1.5">
        <Terminal size={14} className="shrink-0 text-zinc-500" />
        <code className="truncate text-xs text-zinc-400">
          {app.command.join(" ")}
        </code>
      </div>

      {/* Stats */}
      <div className="mb-3 grid grid-cols-3 gap-3 text-xs">
        <div>
          <div className="text-zinc-500">Port</div>
          <div className="font-mono text-zinc-300">{app.port}</div>
        </div>
        <div>
          <div className="text-zinc-500">Uptime</div>
          <div className="text-zinc-300">
            {app.status.type === "running" ? formatUptime(uptimeSeconds) : "-"}
          </div>
        </div>
        <div>
          <div className="text-zinc-500">Restarts</div>
          <div className="text-zinc-300">{app.restarts}</div>
        </div>
      </div>

      {/* Resource usage */}
      {(app.cpu_usage != null || app.memory_bytes != null) && (
        <div className="grid grid-cols-2 gap-3 text-xs">
          <div>
            <div className="text-zinc-500">CPU</div>
            <div className="text-zinc-300">{formatCpu(app.cpu_usage)}</div>
          </div>
          <div>
            <div className="text-zinc-500">Memory</div>
            <div className="text-zinc-300">
              {app.memory_bytes != null
                ? formatBytes(app.memory_bytes)
                : "-"}
            </div>
          </div>
        </div>
      )}

    </div>
  );
}
