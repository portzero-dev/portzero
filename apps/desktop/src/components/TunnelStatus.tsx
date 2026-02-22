import { Globe } from "lucide-react";

export function TunnelStatus() {
  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
      <div className="mb-2 flex items-center gap-2">
        <Globe size={16} className="text-zinc-500" />
        <span className="text-sm font-medium text-zinc-400">
          Public Tunnels
        </span>
        <span className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider text-zinc-500">
          Coming Soon
        </span>
      </div>
      <p className="text-xs text-zinc-500">
        Share apps publicly with a secure tunnel. This feature will be available
        in a future release.
      </p>
    </div>
  );
}
