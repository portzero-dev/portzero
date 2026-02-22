/**
 * Platform-agnostic WsEvent listener.
 *
 * - **Tauri**: uses `listen("ws-event")` from `@tauri-apps/api/event`
 * - **Web**: connects to `/api/ws` via native WebSocket
 *
 * Returns an unlisten function.
 */
import { isTauri, wsUrl } from "../lib/env";
import type { WsEvent } from "../lib/types";

type Callback = (event: WsEvent) => void;

/**
 * Subscribe to WsEvent from either Tauri IPC or the daemon's WebSocket.
 * Returns a cleanup function.
 */
export function listenWsEvent(cb: Callback): () => void {
  if (isTauri()) {
    let unlistenFn: (() => void) | null = null;
    let cancelled = false;

    import("@tauri-apps/api/event").then(({ listen }) => {
      if (cancelled) return;
      listen<WsEvent>("ws-event", (e) => cb(e.payload)).then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlistenFn = fn;
        }
      });
    });

    return () => {
      cancelled = true;
      unlistenFn?.();
    };
  }

  // Web mode: native WebSocket
  let ws: WebSocket | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let cancelled = false;

  function connect() {
    if (cancelled) return;
    ws = new WebSocket(wsUrl());

    ws.onmessage = (msg) => {
      try {
        cb(JSON.parse(msg.data));
      } catch {
        // ignore
      }
    };

    ws.onclose = () => {
      if (!cancelled) {
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
  };
}
