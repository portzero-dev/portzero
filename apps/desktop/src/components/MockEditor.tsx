import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { createMock, updateMock } from "../api/client";
import type { MockRule, CreateMockRule } from "../lib/types";
import { HTTP_METHODS } from "../lib/filters";
import { Save, X } from "lucide-react";

interface MockEditorProps {
  /** If provided, we're editing an existing mock. Otherwise, creating new. */
  existing?: MockRule;
  apps: string[];
  onClose: () => void;
}

export function MockEditor({ existing, apps, onClose }: MockEditorProps) {
  const queryClient = useQueryClient();
  const [appName, setAppName] = useState(existing?.app_name || apps[0] || "");
  const [method, setMethod] = useState(existing?.method || "");
  const [pathPattern, setPathPattern] = useState(
    existing?.path_pattern || "",
  );
  const [statusCode, setStatusCode] = useState(
    existing?.status_code?.toString() || "200",
  );
  const [responseBody, setResponseBody] = useState(
    existing?.response_body || '{"message": "mocked"}',
  );
  const [responseHeaders, setResponseHeaders] = useState(
    JSON.stringify(existing?.response_headers || { "content-type": "application/json" }, null, 2),
  );

  const createMutation = useMutation({
    mutationFn: (rule: CreateMockRule) => createMock(rule),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["mocks"] });
      onClose();
    },
  });

  const updateMutation = useMutation({
    mutationFn: () =>
      updateMock(existing!.id, {
        method: method || undefined,
        path_pattern: pathPattern,
        status_code: parseInt(statusCode),
        response_body: responseBody,
        response_headers: JSON.parse(responseHeaders),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["mocks"] });
      onClose();
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (existing) {
      updateMutation.mutate();
    } else {
      createMutation.mutate({
        app_name: appName,
        method: method || undefined,
        path_pattern: pathPattern,
        status_code: parseInt(statusCode),
        response_body: responseBody,
        response_headers: JSON.parse(responseHeaders),
      });
    }
  };

  const isPending = createMutation.isPending || updateMutation.isPending;

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
      <div className="mb-4 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-zinc-200">
          {existing ? "Edit Mock Rule" : "New Mock Rule"}
        </h3>
        <button
          type="button"
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-300"
        >
          <X size={16} />
        </button>
      </div>

      <form onSubmit={handleSubmit} className="space-y-3">
        {/* App name */}
        <div>
          <label htmlFor="mock-app" className="mb-1 block text-xs text-zinc-500">App</label>
          <input
            id="mock-app"
            type="text"
            list="mock-app-list"
            value={appName}
            onChange={(e) => setAppName(e.target.value)}
            disabled={!!existing}
            placeholder="e.g. outreach-crm"
            required
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-300 placeholder-zinc-600 outline-none focus:border-violet-500 disabled:opacity-50"
          />
          <datalist id="mock-app-list">
            {apps.map((a) => (
              <option key={a} value={a} />
            ))}
          </datalist>
        </div>

        {/* Method + Path */}
        <div className="grid grid-cols-4 gap-2">
          <div>
            <label htmlFor="mock-method" className="mb-1 block text-xs text-zinc-500">Method</label>
            <select
              id="mock-method"
              value={method}
              onChange={(e) => setMethod(e.target.value)}
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-300 outline-none focus:border-violet-500"
            >
              <option value="">Any</option>
              {HTTP_METHODS.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </div>
          <div className="col-span-3">
            <label htmlFor="mock-path" className="mb-1 block text-xs text-zinc-500">
              Path Pattern
            </label>
            <input
              id="mock-path"
              type="text"
              value={pathPattern}
              onChange={(e) => setPathPattern(e.target.value)}
              placeholder="/api/users/*"
              required
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-300 placeholder-zinc-600 outline-none focus:border-violet-500"
            />
          </div>
        </div>

        {/* Status code */}
        <div>
          <label htmlFor="mock-status" className="mb-1 block text-xs text-zinc-500">
            Status Code
          </label>
          <input
            id="mock-status"
            type="number"
            value={statusCode}
            onChange={(e) => setStatusCode(e.target.value)}
            min={100}
            max={599}
            required
            className="w-24 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-300 outline-none focus:border-violet-500"
          />
        </div>

        {/* Response headers */}
        <div>
          <label htmlFor="mock-resp-headers" className="mb-1 block text-xs text-zinc-500">
            Response Headers (JSON)
          </label>
          <textarea
            id="mock-resp-headers"
            value={responseHeaders}
            onChange={(e) => setResponseHeaders(e.target.value)}
            rows={3}
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 font-mono text-xs text-zinc-300 outline-none focus:border-violet-500"
          />
        </div>

        {/* Response body */}
        <div>
          <label htmlFor="mock-resp-body" className="mb-1 block text-xs text-zinc-500">
            Response Body
          </label>
          <textarea
            id="mock-resp-body"
            value={responseBody}
            onChange={(e) => setResponseBody(e.target.value)}
            rows={6}
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 font-mono text-xs text-zinc-300 outline-none focus:border-violet-500"
          />
        </div>

        {/* Submit */}
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg px-3 py-1.5 text-sm text-zinc-400 hover:text-zinc-200"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isPending}
            className="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-violet-700 disabled:opacity-50"
          >
            <Save size={14} />
            {existing ? "Update" : "Create"}
          </button>
        </div>
      </form>
    </div>
  );
}
