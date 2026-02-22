import { useEffect, useRef, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getAppLogs } from "../api/client";
import type { LogLine, WsEvent } from "../lib/types";
import { formatTime } from "../lib/formatters";
import { ArrowDown, Loader2 } from "lucide-react";

interface LogViewerProps {
  appName: string;
}

const POLL_INTERVAL_MS = 2000;

export function LogViewer({ appName }: LogViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const knownCountRef = useRef(0);

  const fetchLogs = useCallback(async () => {
    try {
      const result = await getAppLogs(appName, 500);
      if (result.length > 0 || knownCountRef.current === 0) {
        // Only update if we got data or this is the first fetch
        setLogs(result);
        knownCountRef.current = result.length;
      }
    } catch {
      // Silently ignore — next poll will retry
    } finally {
      setIsLoading(false);
    }
  }, [appName]);

  // Initial load + polling
  useEffect(() => {
    setIsLoading(true);
    setLogs([]);
    knownCountRef.current = 0;
    fetchLogs();

    const interval = setInterval(fetchLogs, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [fetchLogs]);

  // Also listen for Tauri ws-events to trigger immediate re-fetch
  useEffect(() => {
    const unlisten = listen<WsEvent>("ws-event", (tauriEvent) => {
      const event = tauriEvent.payload;
      if (event.type === "log:line" && event.app === appName) {
        fetchLogs();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [appName, fetchLogs]);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs.length, autoScroll]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    setAutoScroll(scrollHeight - scrollTop - clientHeight < 50);
  };

  return (
    <div className="relative">
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="h-80 overflow-auto rounded-lg border border-zinc-800 bg-zinc-950 p-3 font-mono text-xs"
      >
        {isLoading ? (
          <div className="flex h-full items-center justify-center text-zinc-600">
            <Loader2 size={16} className="animate-spin" />
          </div>
        ) : logs.length === 0 ? (
          <div className="flex h-full items-center justify-center text-zinc-600">
            No log output yet
          </div>
        ) : (
          logs.map((line, i) => (
            <div key={`${line.timestamp}-${i}`} className="flex gap-2 leading-5">
              <span className="shrink-0 select-none text-zinc-600">
                {formatTime(line.timestamp)}
              </span>
              <span
                className={
                  line.stream === "stderr" ? "text-red-400" : "text-zinc-300"
                }
              >
                {line.content}
              </span>
            </div>
          ))
        )}
      </div>

      {/* Scroll-to-bottom button */}
      {!autoScroll && (
        <button
          type="button"
          onClick={() => {
            setAutoScroll(true);
            containerRef.current?.scrollTo({
              top: containerRef.current.scrollHeight,
              behavior: "smooth",
            });
          }}
          className="absolute bottom-4 right-4 rounded-full bg-zinc-800 p-1.5 text-zinc-400 shadow-lg hover:bg-zinc-700"
        >
          <ArrowDown size={14} />
        </button>
      )}
    </div>
  );
}
