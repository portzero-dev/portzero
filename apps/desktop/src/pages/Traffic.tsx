import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listRequests, listApps, clearRequests } from "../api/client";
import { RequestRow } from "../components/RequestRow";
import { FilterBar } from "../components/FilterBar";
import type { RequestFilters } from "../lib/types";
import type { Route } from "../App";
import { Loader2, Trash2, AlertCircle, Activity } from "lucide-react";
import { useMutation, useQueryClient } from "@tanstack/react-query";

interface TrafficProps {
  navigate: (route: Route) => void;
}

export function Traffic({ navigate }: TrafficProps) {
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<RequestFilters>({
    limit: 100,
    offset: 0,
  });

  const { data: apps } = useQuery({
    queryKey: ["apps"],
    queryFn: listApps,
  });

  const {
    data: requests,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["requests", filters],
    queryFn: () => listRequests(filters),
  });

  const clearMutation = useMutation({
    mutationFn: () => clearRequests(filters.app),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["requests"] }),
  });

  const appNames = apps?.map((a) => a.name) ?? [];

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-zinc-800 px-6 py-4">
        <div>
          <h1 className="text-xl font-bold text-zinc-100">Traffic</h1>
          <p className="mt-0.5 text-sm text-zinc-500">
            {requests?.length ?? 0} request{requests?.length !== 1 ? "s" : ""}
          </p>
        </div>
        <button
          type="button"
          onClick={() => clearMutation.mutate()}
          disabled={clearMutation.isPending || !requests?.length}
          className="flex items-center gap-1.5 rounded-lg bg-zinc-800 px-3 py-1.5 text-xs text-zinc-400 hover:bg-zinc-700 disabled:opacity-50"
        >
          <Trash2 size={12} />
          Clear
        </button>
      </div>

      {/* Filters */}
      <FilterBar filters={filters} onChange={setFilters} apps={appNames} />

      {/* Request list */}
      <div className="flex-1 overflow-auto">
        {isLoading ? (
          <div className="flex items-center justify-center py-12 text-zinc-500">
            <Loader2 size={20} className="animate-spin" />
          </div>
        ) : error ? (
          <div className="m-4 flex items-center gap-2 rounded-xl border border-red-800/50 bg-red-900/10 px-4 py-3 text-sm text-red-400">
            <AlertCircle size={16} />
            Failed to load requests
          </div>
        ) : requests && requests.length > 0 ? (
          <>
            {/* Column headers */}
            <div className="flex items-center gap-3 border-b border-zinc-800 bg-zinc-900/80 px-4 py-1.5 text-xs font-medium text-zinc-500">
              <span className="w-24 shrink-0">Time</span>
              <span className="w-16 shrink-0">Method</span>
              <span className="w-10 shrink-0">Status</span>
              <span className="min-w-0 flex-1">Path</span>
              <span className="w-24 shrink-0">App</span>
              <span className="w-16 shrink-0 text-right">Duration</span>
              <span className="w-12 shrink-0" />
            </div>
            {requests.map((req) => (
              <RequestRow key={req.id} request={req} navigate={navigate} />
            ))}
            {requests.length >= (filters.limit ?? 100) && (
              <div className="flex justify-center py-4">
                <button
                  type="button"
                  onClick={() =>
                    setFilters((f) => ({
                      ...f,
                      limit: (f.limit ?? 100) + 100,
                    }))
                  }
                  className="text-xs text-violet-400 hover:text-violet-300"
                >
                  Load more
                </button>
              </div>
            )}
          </>
        ) : (
          <div className="flex flex-col items-center justify-center py-16 text-zinc-500">
            <Activity size={32} className="mb-3 opacity-30" />
            <p className="text-sm">No requests captured</p>
            <p className="mt-1 text-xs text-zinc-600">
              Requests will appear here as traffic flows through the proxy
            </p>
          </div>
        )}
      </div>
    </div>
  );
}


