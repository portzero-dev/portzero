import { useQuery } from "@tanstack/react-query";
import { getRequest, diffRequests } from "../api/client";
import { ReplayButton } from "../components/ReplayButton";
import { DiffViewer } from "../components/DiffViewer";
import {
  formatDuration,
  formatTimestamp,
  statusColorClass,
  methodColorClass,
  prettyJson,
} from "../lib/formatters";
import type { Route } from "../App";
import {
  ArrowLeft,
  Loader2,
  AlertCircle,
  Shield,
  Box,
  Clock,
} from "lucide-react";
import { useState, useMemo, useCallback } from "react";

interface RequestDetailPageProps {
  id: string;
  diffId?: string;
  navigate: (route: Route) => void;
}

export function RequestDetailPage({
  id,
  diffId,
  navigate,
}: RequestDetailPageProps) {
  const [activeTab, setActiveTab] = useState<"request" | "response">(
    "request",
  );

  const { data: request, isLoading, error } = useQuery({
    queryKey: ["request", id],
    queryFn: () => getRequest(id),
  });

  const { data: diff } = useQuery({
    queryKey: ["diff", id, diffId],
    queryFn: () => diffRequests(id, diffId!),
    enabled: !!diffId,
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-24 text-zinc-500">
        <Loader2 size={24} className="animate-spin" />
      </div>
    );
  }

  if (error || !request) {
    return (
      <div className="p-6">
        <div className="flex items-center gap-2 rounded-xl border border-red-800/50 bg-red-900/10 px-4 py-3 text-sm text-red-400">
          <AlertCircle size={16} />
          Failed to load request
        </div>
      </div>
    );
  }

  // Diff mode
  if (diff) {
    return (
      <div className="p-6">
        <button
          type="button"
          onClick={() => navigate({ page: "traffic" })}
          className="mb-4 flex items-center gap-1.5 text-sm text-zinc-400 hover:text-zinc-200"
        >
          <ArrowLeft size={14} />
          Back to Traffic
        </button>
        <h1 className="mb-4 text-xl font-bold text-zinc-100">Request Diff</h1>
        <DiffViewer left={diff.left} right={diff.right} />
      </div>
    );
  }

  return (
    <div className="p-6">
      {/* Back button */}
      <button
        type="button"
        onClick={() => navigate({ page: "traffic" })}
        className="mb-4 flex items-center gap-1.5 text-sm text-zinc-400 hover:text-zinc-200"
      >
        <ArrowLeft size={14} />
        Back to Traffic
      </button>

      {/* Header */}
      <div className="mb-6 flex items-start justify-between">
        <div>
          <div className="flex items-center gap-3">
            <span
              className={`font-mono text-lg font-bold ${methodColorClass(request.method)}`}
            >
              {request.method}
            </span>
            <span
              className={`font-mono text-lg font-bold ${statusColorClass(request.status_code)}`}
            >
              {request.status_code} {request.status_message}
            </span>
          </div>
          <p className="mt-1 font-mono text-sm text-zinc-400">
            {request.url}
          </p>
          <div className="mt-2 flex items-center gap-4 text-xs text-zinc-500">
            <span className="flex items-center gap-1">
              <Clock size={12} />
              {formatDuration(request.duration_ms)}
            </span>
            <span>{formatTimestamp(request.timestamp)}</span>
            <span>{request.app_name}</span>
            {request.mocked && (
              <span className="flex items-center gap-1 text-amber-400">
                <Box size={12} />
                Mocked
              </span>
            )}
            {request.intercepted && (
              <span className="flex items-center gap-1 text-violet-400">
                <Shield size={12} />
                Intercepted
              </span>
            )}
            {request.parent_id && (
              <button
                type="button"
                onClick={() =>
                  navigate({ page: "request", id: request.parent_id! })
                }
                className="text-violet-400 hover:text-violet-300"
              >
                Replayed from {request.parent_id.slice(0, 8)}...
              </button>
            )}
          </div>
        </div>
        <ReplayButton requestId={id} navigate={navigate} />
      </div>

      {/* Tabs */}
      <div className="mb-4 flex gap-1 border-b border-zinc-800">
        <TabButton
          active={activeTab === "request"}
          onClick={() => setActiveTab("request")}
          label="Request"
        />
        <TabButton
          active={activeTab === "response"}
          onClick={() => setActiveTab("response")}
          label="Response"
        />
      </div>

      {/* Tab content */}
      {activeTab === "request" ? (
        <div className="space-y-4">
          <HeadersSection
            title="Request Headers"
            headers={request.request_headers}
          />
          {request.request_body && (
            <BodySection
              title="Request Body"
              body={request.request_body}
              contentType={request.request_content_type}
            />
          )}
          {request.query_string && (
            <div>
              <h3 className="mb-2 text-sm font-medium text-zinc-300">
                Query String
              </h3>
              <pre className="rounded-lg border border-zinc-800 bg-zinc-950 p-3 font-mono text-xs text-zinc-400">
                {request.query_string}
              </pre>
            </div>
          )}
        </div>
      ) : (
        <div className="space-y-4">
          <HeadersSection
            title="Response Headers"
            headers={request.response_headers}
          />
          {request.response_body && (
            <BodySection
              title="Response Body"
              body={request.response_body}
              contentType={request.response_content_type}
            />
          )}
        </div>
      )}
    </div>
  );
}

function TabButton({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`border-b-2 px-4 py-2 text-sm font-medium transition-colors ${
        active
          ? "border-violet-500 text-zinc-100"
          : "border-transparent text-zinc-500 hover:text-zinc-300"
      }`}
    >
      {label}
    </button>
  );
}

function HeadersSection({
  title,
  headers,
}: {
  title: string;
  headers: Record<string, string>;
}) {
  const entries = Object.entries(headers).sort(([a], [b]) => a.localeCompare(b));
  return (
    <div>
      <h3 className="mb-2 text-sm font-medium text-zinc-300">{title}</h3>
      <div className="rounded-lg border border-zinc-800 bg-zinc-950 p-3">
        {entries.length === 0 ? (
          <span className="text-xs text-zinc-600">No headers</span>
        ) : (
          <div className="space-y-0.5 font-mono text-xs">
            {entries.map(([key, value]) => (
              <div key={key} className="flex gap-2">
                <span className="shrink-0 text-violet-400">{key}</span>
                <span className="text-zinc-600">:</span>
                <span className="break-all text-zinc-400">{value}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

/** Max characters to render before truncating (roughly 10 KB). */
const BODY_TRUNCATE_THRESHOLD = 10_000;

function BodySection({
  title,
  body,
  contentType,
}: {
  title: string;
  body: string;
  contentType: string | null;
}) {
  const [expanded, setExpanded] = useState(false);

  const isJson = contentType?.includes("json");

  // Memoize the formatted body so we only parse/stringify once.
  const formatted = useMemo(
    () => (isJson ? prettyJson(body) : body),
    [body, isJson],
  );

  const isTruncated = !expanded && formatted.length > BODY_TRUNCATE_THRESHOLD;
  const displayText = isTruncated
    ? formatted.slice(0, BODY_TRUNCATE_THRESHOLD)
    : formatted;

  const handleExpand = useCallback(() => setExpanded(true), []);

  const sizeLabel =
    body.length >= 1024
      ? `${(body.length / 1024).toFixed(1)} KB`
      : `${body.length} B`;

  return (
    <div>
      <div className="mb-2 flex items-center gap-2">
        <h3 className="text-sm font-medium text-zinc-300">{title}</h3>
        {contentType && (
          <span className="text-xs text-zinc-600">{contentType}</span>
        )}
        <span className="text-xs text-zinc-600">{sizeLabel}</span>
      </div>
      <pre className="max-h-96 overflow-auto whitespace-pre-wrap break-all rounded-lg border border-zinc-800 bg-zinc-950 p-3 font-mono text-xs text-zinc-400">
        {displayText}
      </pre>
      {isTruncated && (
        <button
          type="button"
          onClick={handleExpand}
          className="mt-2 text-xs text-violet-400 hover:text-violet-300"
        >
          Show full response ({sizeLabel})
        </button>
      )}
    </div>
  );
}
