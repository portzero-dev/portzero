import type { RequestSummary } from "../lib/types";
import type { Route } from "../App";
import {
  formatDuration,
  formatTime,
  statusColorClass,
  methodColorClass,
  truncate,
} from "../lib/formatters";
import { Box } from "lucide-react";

interface RequestRowProps {
  request: RequestSummary;
  navigate: (route: Route) => void;
}

export function RequestRow({ request, navigate }: RequestRowProps) {
  return (
    <button
      type="button"
      onClick={() => navigate({ page: "request", id: request.id })}
      className="flex w-full items-center gap-3 border-b border-zinc-800/50 px-4 py-2.5 text-left text-sm transition-colors hover:bg-zinc-800/50"
    >
      {/* Time */}
      <span className="w-24 shrink-0 font-mono text-xs text-zinc-500">
        {formatTime(request.timestamp)}
      </span>

      {/* Method */}
      <span
        className={`w-16 shrink-0 font-mono text-xs font-bold ${methodColorClass(request.method)}`}
      >
        {request.method}
      </span>

      {/* Status */}
      <span
        className={`w-10 shrink-0 font-mono text-xs font-bold ${statusColorClass(request.status_code)}`}
      >
        {request.status_code}
      </span>

      {/* Path */}
      <span className="min-w-0 flex-1 truncate font-mono text-xs text-zinc-300">
        {truncate(request.path, 80)}
      </span>

      {/* App name */}
      <span className="w-24 shrink-0 truncate text-xs text-zinc-500">
        {request.app_name}
      </span>

      {/* Duration */}
      <span className="w-16 shrink-0 text-right font-mono text-xs text-zinc-500">
        {formatDuration(request.duration_ms)}
      </span>

      {/* Badges */}
      <div className="flex w-12 shrink-0 items-center justify-end gap-1">
        {request.mocked && (
          <span title="Mocked">
            <Box size={12} className="text-amber-400" />
          </span>
        )}
      </div>
    </button>
  );
}
