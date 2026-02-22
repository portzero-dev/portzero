import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { WsEvent, RequestSummary } from "../lib/types";
import type { Route } from "../App";
import { listRequests } from "../api/client";
import {
  formatDuration,
  formatTime,
  statusColorClass,
  methodColorClass,
  truncate,
} from "../lib/formatters";
import { Radio } from "lucide-react";

const MAX_FEED_ITEMS = 50;
/** Background sync interval to catch any missed events */
const SYNC_INTERVAL_MS = 5000;

interface LiveRequestFeedProps {
  navigate: (route: Route) => void;
}

export function LiveRequestFeed({ navigate }: LiveRequestFeedProps) {
  const [requests, setRequests] = useState<RequestSummary[]>([]);
  const [newIds, setNewIds] = useState<Set<string>>(new Set());
  const [isLive, setIsLive] = useState(true);

  // Full sync from the store — used on mount and as a periodic safety net
  const sync = useCallback(async () => {
    try {
      const results = await listRequests({ limit: MAX_FEED_ITEMS });
      setRequests(results);
    } catch {
      // ignore
    }
  }, []);

  // Initial load
  useEffect(() => {
    sync();
  }, [sync]);

  // Periodic background sync as safety net (catches events that were missed
  // due to race conditions, reconnects, or persistence delays)
  useEffect(() => {
    if (!isLive) return;
    const interval = setInterval(sync, SYNC_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [isLive, sync]);

  // Event-driven updates: listen for request events from the backend
  useEffect(() => {
    if (!isLive) return;

    const unlisten = listen<WsEvent>("ws-event", (tauriEvent) => {
      const event = tauriEvent.payload;

      if (event.type === "request:start") {
        // Extract path from the URL
        let path = event.url;
        try {
          path = new URL(event.url).pathname;
        } catch {
          // If URL parsing fails, use raw value
        }

        const stub: RequestSummary = {
          id: event.id,
          app_name: event.app,
          method: event.method,
          path,
          timestamp: event.timestamp,
          status_code: 0,
          duration_ms: 0,
          mocked: false,
        };

        setRequests((prev) => {
          // Prepend the in-flight entry and cap at MAX_FEED_ITEMS
          const next = [stub, ...prev];
          return next.slice(0, MAX_FEED_ITEMS);
        });

        // Mark as new for animation
        setNewIds((prev) => new Set(prev).add(event.id));
        setTimeout(() => {
          setNewIds((prev) => {
            const next = new Set(prev);
            next.delete(event.id);
            return next;
          });
        }, 1200);
      }

      if (event.type === "request:complete") {
        // Update the in-flight entry in place with final status + duration
        setRequests((prev) =>
          prev.map((r) =>
            r.id === event.id
              ? { ...r, status_code: event.status, duration_ms: event.duration_ms }
              : r,
          ),
        );
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isLive]);

  return (
    <div className="flex flex-col overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900">
      {/* Feed header */}
      <div className="flex items-center justify-between border-b border-zinc-800 px-4 py-2.5">
        <div className="flex items-center gap-2">
          <div className="relative flex items-center">
            {isLive && (
              <span className="absolute inline-flex h-2.5 w-2.5 animate-ping rounded-full bg-emerald-400 opacity-75" />
            )}
            <span
              className={`relative inline-flex h-2.5 w-2.5 rounded-full ${
                isLive ? "bg-emerald-400" : "bg-zinc-600"
              }`}
            />
          </div>
          <span className="text-xs font-medium text-zinc-300">
            Live Requests
          </span>
          {requests.length > 0 && (
            <span className="text-[10px] text-zinc-600">
              {requests.length} shown
            </span>
          )}
        </div>
        <button
          type="button"
          onClick={() => setIsLive((v) => !v)}
          className={`flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium transition-colors ${
            isLive
              ? "bg-emerald-500/10 text-emerald-400 hover:bg-emerald-500/20"
              : "bg-zinc-800 text-zinc-500 hover:bg-zinc-700"
          }`}
        >
          <Radio size={10} />
          {isLive ? "Pause" : "Resume"}
        </button>
      </div>

      {/* Column headers */}
      <div className="flex items-center gap-3 border-b border-zinc-800/50 bg-zinc-900/80 px-4 py-1.5 text-[11px] font-medium text-zinc-600">
        <span className="w-20 shrink-0">Time</span>
        <span className="w-14 shrink-0">Method</span>
        <span className="w-10 shrink-0">Status</span>
        <span className="min-w-0 flex-1">Path</span>
        <span className="w-20 shrink-0">App</span>
        <span className="w-14 shrink-0 text-right">Duration</span>
      </div>

      {/* Feed list */}
      <div className="max-h-[400px] overflow-auto">
        {requests.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-10 text-zinc-600">
            <Radio size={24} className="mb-2 opacity-30" />
            <p className="text-xs">Waiting for requests...</p>
            <p className="mt-0.5 text-[11px] text-zinc-700">
              Requests will appear here as traffic flows through the proxy
            </p>
          </div>
        ) : (
          requests.map((req) => (
            <FeedRow
              key={req.id}
              request={req}
              isNew={newIds.has(req.id)}
              navigate={navigate}
            />
          ))
        )}
      </div>
    </div>
  );
}

function FeedRow({
  request,
  isNew,
  navigate,
}: {
  request: RequestSummary;
  isNew: boolean;
  navigate: (route: Route) => void;
}) {
  const isInFlight = request.status_code === 0;

  return (
    <button
      type="button"
      onClick={() => !isInFlight && navigate({ page: "request", id: request.id })}
      className={`flex w-full items-center gap-3 border-b border-zinc-800/30 px-4 py-2 text-left text-sm transition-all hover:bg-zinc-800/50 ${
        isNew ? "animate-feed-in bg-emerald-500/5" : ""
      } ${isInFlight ? "opacity-50" : ""}`}
    >
      <span className="w-20 shrink-0 font-mono text-[11px] text-zinc-500">
        {formatTime(request.timestamp)}
      </span>
      <span
        className={`w-14 shrink-0 font-mono text-[11px] font-bold ${methodColorClass(request.method)}`}
      >
        {request.method}
      </span>
      <span
        className={`w-10 shrink-0 font-mono text-[11px] font-bold ${
          isInFlight ? "text-zinc-600" : statusColorClass(request.status_code)
        }`}
      >
        {isInFlight ? "..." : request.status_code}
      </span>
      <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-300">
        {truncate(request.path, 60)}
      </span>
      <span className="w-20 shrink-0 truncate text-[11px] text-zinc-500">
        {request.app_name}
      </span>
      <span className="w-14 shrink-0 text-right font-mono text-[11px] text-zinc-500">
        {isInFlight ? "..." : formatDuration(request.duration_ms)}
      </span>
    </button>
  );
}
