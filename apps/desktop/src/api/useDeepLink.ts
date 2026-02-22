/**
 * Deep link handler for the `portzero://` URL scheme.
 *
 * Listens for deep link events from the Tauri backend and translates
 * them into in-app navigation.
 *
 * Supported URLs:
 *   portzero://                      → overview
 *   portzero://apps/<name>           → app detail
 *   portzero://traffic               → traffic list
 *   portzero://traffic/<id>          → request detail
 *   portzero://mocks                 → mocks
 *   portzero://settings              → settings
 */

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import type { Route } from "../App";

/**
 * Parse a `portzero://` URL into an in-app Route.
 */
function parseDeepLinkUrl(url: string): Route | null {
  try {
    // portzero://apps/my-app → URL with host = "apps", path = "/my-app"
    // portzero://traffic/123 → URL with host = "traffic", path = "/123"
    const parsed = new URL(url);

    // The scheme is "portzero:", host is the first segment
    const host = parsed.hostname || parsed.pathname.replace(/^\/+/, "").split("/")[0];
    const pathParts = parsed.pathname
      .split("/")
      .filter(Boolean);

    switch (host) {
      case "apps":
        if (pathParts[0]) {
          return { page: "app", name: decodeURIComponent(pathParts[0]) };
        }
        return { page: "overview" };

      case "traffic":
        if (pathParts[0]) {
          return { page: "request", id: pathParts[0] };
        }
        return { page: "traffic" };

      case "mocks":
        return { page: "mocks" };

      case "settings":
        return { page: "settings" };

      default:
        return { page: "overview" };
    }
  } catch {
    return null;
  }
}

/**
 * React hook that handles `portzero://` deep link navigation.
 *
 * Call this in your root App component, passing the navigate function.
 */
export function useDeepLink(navigate: (route: Route) => void) {
  useEffect(() => {
    // Check if the app was opened via a deep link at startup
    getCurrent()
      .then((urls) => {
        if (urls && urls.length > 0) {
          const route = parseDeepLinkUrl(urls[0]);
          if (route) navigate(route);
        }
      })
      .catch(() => {
        // Ignore — deep-link plugin may not be available in dev
      });

    // Listen for deep links while the app is running (from Rust on_open_url)
    const unlistenTauri = listen<string[]>("deep-link", (event) => {
      const urls = event.payload;
      if (urls && urls.length > 0) {
        const route = parseDeepLinkUrl(urls[0]);
        if (route) navigate(route);
      }
    });

    // Also listen via the JS plugin API (covers all platforms)
    const unlistenPlugin = onOpenUrl((urls) => {
      if (urls && urls.length > 0) {
        const route = parseDeepLinkUrl(urls[0].toString());
        if (route) navigate(route);
      }
    });

    return () => {
      unlistenTauri.then((fn) => fn());
      unlistenPlugin.then((fn) => fn());
    };
  }, [navigate]);
}
