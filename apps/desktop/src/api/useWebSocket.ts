import { useEffect, useCallback, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import type { WsEvent } from "../lib/types";

type WsEventHandler = (event: WsEvent) => void;

/**
 * Subscribe to real-time events from the Tauri backend.
 *
 * The Rust backend forwards events as Tauri `"ws-event"` events from two sources:
 * 1. The local WsHub (for apps managed by the desktop app)
 * 2. The daemon's event stream via control socket subscription (for CLI-managed apps)
 *
 * The listener is set up once and stays stable — the `onEvent` callback is
 * stored in a ref so it can change without re-subscribing.
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

  useEffect(() => {
    setConnected(true);

    const unlistenWs = listen<WsEvent>("ws-event", (tauriEvent) => {
      const event = tauriEvent.payload;
      invalidateQueries(event);
      onEventRef.current?.(event);
    });

    // Listen for daemon state changes triggered by tray menu actions
    const unlistenDaemon = listen("daemon-state-changed", () => {
      queryClient.invalidateQueries({ queryKey: ["daemon-info"] });
      queryClient.invalidateQueries({ queryKey: ["status"] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    });

    return () => {
      unlistenWs.then((fn) => fn());
      unlistenDaemon.then((fn) => fn());
      setConnected(false);
    };
  }, [invalidateQueries, queryClient]);

  return { connected };
}
