import { useEffect, useCallback, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { isTauri, wsUrl } from "../lib/env";
import type { WsEvent } from "../lib/types";

type WsEventHandler = (event: WsEvent) => void;

/**
 * Subscribe to real-time events.
 *
 * - **Tauri mode**: listens to `"ws-event"` Tauri events forwarded by the Rust backend.
 * - **Web mode**: connects to the daemon's WebSocket endpoint at `/api/ws`.
 */
export function usePortZeroWebSocket(onEvent?: WsEventHandler) {
  const queryClient = useQueryClient();
  const [connected, setConnected] = useState(false);
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  const invalidateQueries = useCallback(
    (event: WsEvent) => {
      switch (event.type) {
        case "request:start":
        case "request:complete":
          queryClient.invalidateQueries({ queryKey: ["requests"] });
          queryClient.invalidateQueries({ queryKey: ["status"] });
          break;

        case "app:registered":
        case "app:removed":
        case "app:crashed":
        case "app:restarted":
          queryClient.invalidateQueries({ queryKey: ["apps"] });
          queryClient.invalidateQueries({ queryKey: ["status"] });
          if ("name" in event) {
            queryClient.invalidateQueries({
              queryKey: ["app", event.name],
            });
          }
          break;

        case "mock:hit":
          queryClient.invalidateQueries({ queryKey: ["mocks"] });
          break;

        case "intercept:pending":
        case "intercept:decided":
          queryClient.invalidateQueries({
            queryKey: ["intercepts"],
          });
          break;

        case "tunnel:started":
        case "tunnel:stopped":
          queryClient.invalidateQueries({ queryKey: ["apps"] });
          if ("app" in event) {
            queryClient.invalidateQueries({
              queryKey: ["app", event.app],
            });
          }
          break;
      }
    },
    [queryClient],
  );

  // Tauri mode: listen to Tauri events
  useEffect(() => {
    if (!isTauri()) return;

    setConnected(true);

    let cancelled = false;

    // Dynamic import to avoid bundling Tauri APIs in web builds
    import("@tauri-apps/api/event").then(({ listen }) => {
      if (cancelled) return;

      const unlistenWs = listen<WsEvent>("ws-event", (tauriEvent) => {
        const event = tauriEvent.payload;
        invalidateQueries(event);
        onEventRef.current?.(event);
      });

      const unlistenDaemon = listen("daemon-state-changed", () => {
        queryClient.invalidateQueries({ queryKey: ["daemon-info"] });
        queryClient.invalidateQueries({ queryKey: ["status"] });
        queryClient.invalidateQueries({ queryKey: ["apps"] });
      });

      // Store unlisten fns for cleanup
      Promise.all([unlistenWs, unlistenDaemon]).then(([unwsF, undF]) => {
        if (cancelled) {
          unwsF();
          undF();
        } else {
          cleanupRef.current = () => {
            unwsF();
            undF();
          };
        }
      });
    });

    const cleanupRef = { current: () => {} };

    return () => {
      cancelled = true;
      cleanupRef.current();
      setConnected(false);
    };
  }, [invalidateQueries, queryClient]);

  // Web mode: native WebSocket
  useEffect(() => {
    if (isTauri()) return;

    let ws: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    let cancelled = false;

    function connect() {
      if (cancelled) return;

      ws = new WebSocket(wsUrl());

      ws.onopen = () => {
        setConnected(true);
      };

      ws.onmessage = (msg) => {
        try {
          const event: WsEvent = JSON.parse(msg.data);
          invalidateQueries(event);
          onEventRef.current?.(event);
        } catch {
          // Ignore malformed messages
        }
      };

      ws.onclose = () => {
        setConnected(false);
        if (!cancelled) {
          // Reconnect after a short delay
          reconnectTimer = setTimeout(connect, 2000);
        }
      };

      ws.onerror = () => {
        ws?.close();
      };
    }

    connect();

    return () => {
      cancelled = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      ws?.close();
      setConnected(false);
    };
  }, [invalidateQueries]);

  return { connected };
}
