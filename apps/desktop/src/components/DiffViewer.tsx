import type { RequestDetail } from "../lib/types";
import {
  statusColorClass,
  methodColorClass,
  formatDuration,
  prettyJson,
} from "../lib/formatters";

interface DiffViewerProps {
  left: RequestDetail;
  right: RequestDetail;
}

export function DiffViewer({ left, right }: DiffViewerProps) {
  return (
    <div className="space-y-4">
      {/* Summary row */}
      <div className="grid grid-cols-2 gap-4">
        <DiffSummary label="Left" request={left} />
        <DiffSummary label="Right" request={right} />
      </div>

      {/* Headers diff */}
      <div>
        <h4 className="mb-2 text-sm font-medium text-zinc-300">
          Request Headers
        </h4>
        <div className="grid grid-cols-2 gap-4">
          <HeadersPanel headers={left.request_headers} />
          <HeadersPanel headers={right.request_headers} />
        </div>
      </div>

      {/* Request body diff */}
      {(left.request_body || right.request_body) && (
        <div>
          <h4 className="mb-2 text-sm font-medium text-zinc-300">
            Request Body
          </h4>
          <div className="grid grid-cols-2 gap-4">
            <BodyPanel body={left.request_body} />
            <BodyPanel body={right.request_body} />
          </div>
        </div>
      )}

      {/* Response headers diff */}
      <div>
        <h4 className="mb-2 text-sm font-medium text-zinc-300">
          Response Headers
        </h4>
        <div className="grid grid-cols-2 gap-4">
          <HeadersPanel headers={left.response_headers} />
          <HeadersPanel headers={right.response_headers} />
        </div>
      </div>

      {/* Response body diff */}
      {(left.response_body || right.response_body) && (
        <div>
          <h4 className="mb-2 text-sm font-medium text-zinc-300">
            Response Body
          </h4>
          <div className="grid grid-cols-2 gap-4">
            <BodyPanel body={left.response_body} />
            <BodyPanel body={right.response_body} />
          </div>
        </div>
      )}
    </div>
  );
}

function DiffSummary({
  label,
  request,
}: {
  label: string;
  request: RequestDetail;
}) {
  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900 p-3">
      <div className="mb-1 text-xs text-zinc-500">{label}</div>
      <div className="flex items-center gap-2 text-sm">
        <span className={`font-mono font-bold ${methodColorClass(request.method)}`}>
          {request.method}
        </span>
        <span className={`font-mono font-bold ${statusColorClass(request.status_code)}`}>
          {request.status_code}
        </span>
        <span className="truncate text-zinc-400">{request.path}</span>
        <span className="text-zinc-500">
          {formatDuration(request.duration_ms)}
        </span>
      </div>
    </div>
  );
}

function HeadersPanel({ headers }: { headers: Record<string, string> }) {
  const entries = Object.entries(headers);
  return (
    <div className="max-h-60 overflow-auto rounded-lg border border-zinc-800 bg-zinc-950 p-3">
      {entries.length === 0 ? (
        <span className="text-xs text-zinc-600">No headers</span>
      ) : (
        <div className="space-y-0.5 font-mono text-xs">
          {entries.map(([key, value]) => (
            <div key={key}>
              <span className="text-violet-400">{key}</span>
              <span className="text-zinc-600">: </span>
              <span className="text-zinc-400">{value}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function BodyPanel({ body }: { body: string | null }) {
  return (
    <div className="max-h-80 overflow-auto rounded-lg border border-zinc-800 bg-zinc-950 p-3">
      {body ? (
        <pre className="whitespace-pre-wrap font-mono text-xs text-zinc-400">
          {prettyJson(body)}
        </pre>
      ) : (
        <span className="text-xs text-zinc-600">No body</span>
      )}
    </div>
  );
}
