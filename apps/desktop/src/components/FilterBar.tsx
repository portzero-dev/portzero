import type { RequestFilters } from "../lib/types";
import { HTTP_METHODS, STATUS_RANGES } from "../lib/filters";
import { Search, X } from "lucide-react";

interface FilterBarProps {
  filters: RequestFilters;
  onChange: (filters: RequestFilters) => void;
  apps: string[];
}

export function FilterBar({ filters, onChange, apps }: FilterBarProps) {
  const update = (patch: Partial<RequestFilters>) =>
    onChange({ ...filters, ...patch });

  const hasFilters =
    filters.app ||
    filters.method ||
    filters.status_range ||
    filters.search;

  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-zinc-800 bg-zinc-900/50 px-4 py-2.5">
      {/* Search */}
      <div className="relative flex-1">
        <Search
          size={14}
          className="absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-500"
        />
        <input
          type="text"
          placeholder="Search requests..."
          value={filters.search || ""}
          onChange={(e) => update({ search: e.target.value || undefined })}
          className="w-full rounded-lg border border-zinc-700 bg-zinc-800 py-1.5 pl-8 pr-3 text-sm text-zinc-300 placeholder-zinc-500 outline-none focus:border-violet-500"
        />
      </div>

      {/* App filter */}
      {apps.length > 1 && (
        <select
          value={filters.app || ""}
          onChange={(e) => update({ app: e.target.value || undefined })}
          className="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-300 outline-none focus:border-violet-500"
        >
          <option value="">All apps</option>
          {apps.map((app) => (
            <option key={app} value={app}>
              {app}
            </option>
          ))}
        </select>
      )}

      {/* Method filter */}
      <select
        value={filters.method || ""}
        onChange={(e) => update({ method: e.target.value || undefined })}
        className="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-300 outline-none focus:border-violet-500"
      >
        <option value="">All methods</option>
        {HTTP_METHODS.map((m) => (
          <option key={m} value={m}>
            {m}
          </option>
        ))}
      </select>

      {/* Status range filter */}
      <select
        value={filters.status_range || ""}
        onChange={(e) =>
          update({ status_range: e.target.value || undefined })
        }
        className="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-300 outline-none focus:border-violet-500"
      >
        {STATUS_RANGES.map((s) => (
          <option key={s.value} value={s.value}>
            {s.label}
          </option>
        ))}
      </select>

      {/* Clear filters */}
      {hasFilters && (
        <button
          type="button"
          onClick={() =>
            onChange({
              limit: filters.limit,
              offset: filters.offset,
            })
          }
          className="flex items-center gap-1 rounded-lg px-2 py-1.5 text-xs text-zinc-400 hover:text-zinc-200"
        >
          <X size={12} />
          Clear
        </button>
      )}
    </div>
  );
}
