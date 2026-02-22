import { useMutation } from "@tanstack/react-query";
import { replayRequest } from "../api/client";
import { Play, Loader2 } from "lucide-react";
import type { Route } from "../App";

interface ReplayButtonProps {
  requestId: string;
  navigate: (route: Route) => void;
}

export function ReplayButton({ requestId, navigate }: ReplayButtonProps) {
  const mutation = useMutation({
    mutationFn: () => replayRequest(requestId),
    onSuccess: (data) => {
      navigate({ page: "request", id: data.id });
    },
  });

  return (
    <button
      type="button"
      onClick={() => mutation.mutate()}
      disabled={mutation.isPending}
      className="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-violet-700 disabled:opacity-50"
    >
      {mutation.isPending ? (
        <Loader2 size={14} className="animate-spin" />
      ) : (
        <Play size={14} />
      )}
      Replay
    </button>
  );
}
