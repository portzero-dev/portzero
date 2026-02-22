import { useQuery } from "@tanstack/react-query";
import { getAppSchema } from "../api/client";
import type { InferredEndpoint, JsonSchema } from "../lib/types";
import { methodColorClass } from "../lib/formatters";
import { Loader2, ChevronRight } from "lucide-react";
import { useState } from "react";

interface SchemaViewerProps {
  appName: string;
}

export function SchemaViewer({ appName }: SchemaViewerProps) {
  const { data: schema, isLoading } = useQuery({
    queryKey: ["schema", appName],
    queryFn: () => getAppSchema(appName),
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8 text-zinc-500">
        <Loader2 size={16} className="animate-spin" />
      </div>
    );
  }

  if (!schema || schema.endpoints.length === 0) {
    return (
      <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-6 text-center text-sm text-zinc-500">
        No API schema inferred yet. Traffic will be analyzed automatically.
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-zinc-200">
          Inferred API Schema
        </h3>
        <span className="text-xs text-zinc-500">
          {schema.endpoints.length} endpoint
          {schema.endpoints.length !== 1 ? "s" : ""}
        </span>
      </div>
      {schema.endpoints.map((endpoint) => (
        <EndpointRow key={`${endpoint.method}-${endpoint.path_template}`} endpoint={endpoint} />
      ))}
    </div>
  );
}

function EndpointRow({ endpoint }: { endpoint: InferredEndpoint }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex w-full items-center gap-3 px-3 py-2 text-left text-sm"
      >
        <ChevronRight
          size={14}
          className={`shrink-0 text-zinc-500 transition-transform ${expanded ? "rotate-90" : ""}`}
        />
        <span className={`w-14 shrink-0 font-mono text-xs font-bold ${methodColorClass(endpoint.method)}`}>
          {endpoint.method}
        </span>
        <span className="flex-1 truncate font-mono text-xs text-zinc-300">
          {endpoint.path_template}
        </span>
        <span className="text-xs text-zinc-500">
          {endpoint.sample_count} sample{endpoint.sample_count !== 1 ? "s" : ""}
        </span>
      </button>

      {expanded && (
        <div className="border-t border-zinc-800 px-3 py-3 text-xs">
          {/* Query params */}
          {endpoint.query_params.length > 0 && (
            <div className="mb-3">
              <span className="mb-1 block font-medium text-zinc-400">
                Query Parameters
              </span>
              <div className="space-y-1">
                {endpoint.query_params.map((p) => (
                  <div key={p.name} className="flex gap-2 font-mono">
                    <span className="text-violet-400">{p.name}</span>
                    <span className="text-zinc-600">: {p.type}</span>
                    {p.required && (
                      <span className="text-red-400">*</span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Request body schema */}
          {endpoint.request_body_schema && (
            <div className="mb-3">
              <span className="mb-1 block font-medium text-zinc-400">
                Request Body
              </span>
              <SchemaTree schema={endpoint.request_body_schema} />
            </div>
          )}

          {/* Response schemas */}
          {Object.entries(endpoint.response_schemas).map(([status, resSchema]) => (
            <div key={status} className="mb-3">
              <span className="mb-1 block font-medium text-zinc-400">
                Response {status}
              </span>
              <SchemaTree schema={resSchema} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SchemaTree({
  schema,
  depth = 0,
}: {
  schema: JsonSchema;
  depth?: number;
}) {
  const indent = depth * 16;

  if (schema.type === "object" && schema.properties) {
    return (
      <div className="font-mono">
        {Object.entries(schema.properties).map(([key, prop]) => (
          <div key={key} style={{ paddingLeft: indent }}>
            <span className="text-violet-400">{key}</span>
            <span className="text-zinc-600">
              : {prop.type}
              {schema.required?.includes(key) ? "" : "?"}
            </span>
            {prop.type === "object" && prop.properties && (
              <SchemaTree schema={prop} depth={depth + 1} />
            )}
          </div>
        ))}
      </div>
    );
  }

  if (schema.type === "array" && schema.items) {
    return (
      <div className="font-mono" style={{ paddingLeft: indent }}>
        <span className="text-zinc-600">
          {schema.items.type}[]
        </span>
        {schema.items.type === "object" && schema.items.properties && (
          <SchemaTree schema={schema.items} depth={depth + 1} />
        )}
      </div>
    );
  }

  return (
    <span className="font-mono text-zinc-600" style={{ paddingLeft: indent }}>
      {schema.type}
    </span>
  );
}
