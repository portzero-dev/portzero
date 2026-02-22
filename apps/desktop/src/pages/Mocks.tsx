import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listMocks, listApps, deleteMock, toggleMock } from "../api/client";
import { MockEditor } from "../components/MockEditor";
import { methodColorClass, statusColorClass } from "../lib/formatters";
import {
  Plus,
  Loader2,
  AlertCircle,
  Trash2,
  Pencil,
  ToggleLeft,
  ToggleRight,
} from "lucide-react";
import type { MockRule } from "../lib/types";

export function Mocks() {
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState<MockRule | "new" | null>(null);

  const { data: mocks, isLoading, error } = useQuery({
    queryKey: ["mocks"],
    queryFn: listMocks,
  });

  const { data: apps } = useQuery({
    queryKey: ["apps"],
    queryFn: listApps,
  });

  const deleteMutation = useMutation({
    mutationFn: deleteMock,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["mocks"] }),
  });

  const toggleMutation = useMutation({
    mutationFn: toggleMock,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["mocks"] }),
  });

  const appNames = apps?.map((a) => a.name) ?? [];

  return (
    <div className="p-6">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-zinc-100">Mocks</h1>
          <p className="mt-0.5 text-sm text-zinc-500">
            Create mock responses for API endpoints
          </p>
        </div>
        <button
          type="button"
          onClick={() => setEditing("new")}
          className="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-violet-700"
        >
          <Plus size={14} />
          New Mock
        </button>
      </div>

      {/* Editor */}
      {editing && (
        <div className="mb-6">
          <MockEditor
            existing={editing === "new" ? undefined : editing}
            apps={appNames}
            onClose={() => setEditing(null)}
          />
        </div>
      )}

      {/* Mock list */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12 text-zinc-500">
          <Loader2 size={20} className="animate-spin" />
        </div>
      ) : error ? (
        <div className="flex items-center gap-2 rounded-xl border border-red-800/50 bg-red-900/10 px-4 py-3 text-sm text-red-400">
          <AlertCircle size={16} />
          Failed to load mocks
        </div>
      ) : mocks && mocks.length > 0 ? (
        <div className="space-y-2">
          {mocks.map((mock) => (
            <div
              key={mock.id}
              className={`flex items-center gap-3 rounded-xl border bg-zinc-900 px-4 py-3 ${
                mock.enabled
                  ? "border-zinc-800"
                  : "border-zinc-800/50 opacity-60"
              }`}
            >
              {/* Toggle */}
              <button
                type="button"
                onClick={() => toggleMutation.mutate(mock.id)}
                className="text-zinc-400 hover:text-zinc-200"
                title={mock.enabled ? "Disable" : "Enable"}
              >
                {mock.enabled ? (
                  <ToggleRight size={20} className="text-violet-400" />
                ) : (
                  <ToggleLeft size={20} />
                )}
              </button>

              {/* Method */}
              <span
                className={`w-14 shrink-0 font-mono text-xs font-bold ${
                  mock.method
                    ? methodColorClass(mock.method)
                    : "text-zinc-500"
                }`}
              >
                {mock.method || "ANY"}
              </span>

              {/* Path pattern */}
              <span className="min-w-0 flex-1 truncate font-mono text-sm text-zinc-300">
                {mock.path_pattern}
              </span>

              {/* Status */}
              <span
                className={`font-mono text-xs font-bold ${statusColorClass(mock.status_code)}`}
              >
                {mock.status_code}
              </span>

              {/* App */}
              <span className="w-20 shrink-0 truncate text-xs text-zinc-500">
                {mock.app_name}
              </span>

              {/* Hit count */}
              <span className="w-14 shrink-0 text-right text-xs text-zinc-500">
                {mock.hit_count} hit{mock.hit_count !== 1 ? "s" : ""}
              </span>

              {/* Actions */}
              <div className="flex gap-1">
                <button
                  type="button"
                  onClick={() => setEditing(mock)}
                  className="rounded p-1 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300"
                  title="Edit"
                >
                  <Pencil size={14} />
                </button>
                <button
                  type="button"
                  onClick={() => deleteMutation.mutate(mock.id)}
                  className="rounded p-1 text-zinc-500 hover:bg-zinc-800 hover:text-red-400"
                  title="Delete"
                >
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-8 text-center">
          <p className="text-sm text-zinc-500">No mock rules defined</p>
          <p className="mt-1 text-xs text-zinc-600">
            Create a mock to intercept requests and return custom responses
          </p>
        </div>
      )}
    </div>
  );
}
