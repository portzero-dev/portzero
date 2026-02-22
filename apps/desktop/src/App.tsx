import { useState, useCallback } from "react";
import { Layout } from "./components/Layout";
import { Overview } from "./pages/Overview";
import { Traffic } from "./pages/Traffic";
import { RequestDetailPage } from "./pages/RequestDetail";
import { AppDetailPage } from "./pages/AppDetail";
import { Mocks } from "./pages/Mocks";
import { Settings } from "./pages/Settings";
import { usePortZeroWebSocket } from "./api/useWebSocket";
import { useDeepLink } from "./api/useDeepLink";
import type { WsEvent } from "./lib/types";

// Simple hash-based router for desktop app / embedded SPA context.
// This avoids TanStack Router's file-based setup complexity while giving
// us type-safe navigation. We can upgrade later if needed.

export type Route =
  | { page: "overview" }
  | { page: "traffic"; requestId?: string }
  | { page: "request"; id: string }
  | { page: "app"; name: string }
  | { page: "mocks" }
  | { page: "settings" }
  | { page: "diff"; id1: string; id2: string };

function parseHash(): Route {
  const hash = window.location.hash.slice(1) || "/";
  const parts = hash.split("/").filter(Boolean);

  if (parts[0] === "traffic" && parts[1]) {
    return { page: "request", id: parts[1] };
  }
  if (parts[0] === "traffic") {
    return { page: "traffic" };
  }
  if (parts[0] === "apps" && parts[1]) {
    return { page: "app", name: decodeURIComponent(parts[1]) };
  }
  if (parts[0] === "mocks") {
    return { page: "mocks" };
  }
  if (parts[0] === "settings") {
    return { page: "settings" };
  }
  if (parts[0] === "diff" && parts[1] && parts[2]) {
    return { page: "diff", id1: parts[1], id2: parts[2] };
  }

  return { page: "overview" };
}

export function useNavigate() {
  return useCallback((route: Route) => {
    switch (route.page) {
      case "overview":
        window.location.hash = "/";
        break;
      case "traffic":
        window.location.hash = "/traffic";
        break;
      case "request":
        window.location.hash = `/traffic/${route.id}`;
        break;
      case "app":
        window.location.hash = `/apps/${encodeURIComponent(route.name)}`;
        break;
      case "mocks":
        window.location.hash = "/mocks";
        break;
      case "settings":
        window.location.hash = "/settings";
        break;
      case "diff":
        window.location.hash = `/diff/${route.id1}/${route.id2}`;
        break;
    }
  }, []);
}

export function App() {
  const [route, setRoute] = useState<Route>(parseHash);
  const { connected } = usePortZeroWebSocket((_event: WsEvent) => {
    // Event-specific handling can be added here if needed.
    // TanStack Query invalidation already happens in the hook.
  });

  // Listen for hash changes
  useState(() => {
    const handler = () => setRoute(parseHash());
    window.addEventListener("hashchange", handler);
    return () => window.removeEventListener("hashchange", handler);
  });

  const navigate = useNavigate();

  // Handle portzero:// deep link navigation
  useDeepLink(navigate);

  return (
    <Layout route={route} navigate={navigate} connected={connected}>
      {renderPage(route, navigate)}
    </Layout>
  );
}

function renderPage(
  route: Route,
  navigate: (route: Route) => void,
) {
  switch (route.page) {
    case "overview":
      return <Overview navigate={navigate} />;
    case "traffic":
      return <Traffic navigate={navigate} />;
    case "request":
      return <RequestDetailPage id={route.id} navigate={navigate} />;
    case "app":
      return <AppDetailPage name={route.name} navigate={navigate} />;
    case "mocks":
      return <Mocks />;
    case "settings":
      return <Settings />;
    case "diff":
      return <RequestDetailPage id={route.id1} diffId={route.id2} navigate={navigate} />;
    default:
      return <Overview navigate={navigate} />;
  }
}
