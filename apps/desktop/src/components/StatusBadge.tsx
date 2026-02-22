import type { AppStatus } from "../lib/types";

interface StatusBadgeProps {
  status: AppStatus;
  className?: string;
}

export function StatusBadge({ status, className = "" }: StatusBadgeProps) {
  const { label, dotClass, bgClass } = getStatusStyles(status ?? { type: "stopped" });

  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium ${bgClass} ${className}`}
    >
      <span className={`h-1.5 w-1.5 rounded-full ${dotClass}`} />
      {label}
    </span>
  );
}

function getStatusStyles(status: AppStatus) {
  switch (status.type) {
    case "running":
      return {
        label: "Running",
        dotClass: "bg-emerald-400",
        bgClass: "bg-emerald-400/10 text-emerald-400",
      };
    case "crashed":
      return {
        label: `Crashed (${status.exit_code})`,
        dotClass: "bg-red-400",
        bgClass: "bg-red-400/10 text-red-400",
      };
    case "stopped":
      return {
        label: "Stopped",
        dotClass: "bg-zinc-500",
        bgClass: "bg-zinc-500/10 text-zinc-500",
      };
  }
}
