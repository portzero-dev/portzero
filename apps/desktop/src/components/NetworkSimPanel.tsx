import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  getNetworkProfile,
  updateNetworkProfile,
  clearNetworkProfile,
} from "../api/client";
import type { UpdateNetworkProfile } from "../lib/types";
import { Loader2, Trash2, Save, Wifi, ChevronDown, Info } from "lucide-react";

interface NetworkSimPanelProps {
  appName: string;
}

export function NetworkSimPanel({ appName }: NetworkSimPanelProps) {
  const queryClient = useQueryClient();
  const [expanded, setExpanded] = useState(false);

  const { data: profile, isLoading } = useQuery({
    queryKey: ["network", appName],
    queryFn: () => getNetworkProfile(appName),
  });

  const [latency, setLatency] = useState(0);
  const [jitter, setJitter] = useState(0);
  const [lossRate, setLossRate] = useState(0);
  const [bandwidth, setBandwidth] = useState(0);

  useEffect(() => {
    if (profile) {
      setLatency(profile.latency_ms ?? 0);
      setJitter(profile.latency_jitter_ms ?? 0);
      setLossRate(Math.round(profile.packet_loss_rate * 100));
      setBandwidth(
        profile.bandwidth_limit_bytes
          ? Math.round(profile.bandwidth_limit_bytes / 1024)
          : 0,
      );
    }
  }, [profile]);

  const updateMutation = useMutation({
    mutationFn: (p: UpdateNetworkProfile) =>
      updateNetworkProfile(appName, p),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["network", appName] }),
  });

  const clearMutation = useMutation({
    mutationFn: () => clearNetworkProfile(appName),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["network", appName] }),
  });

  const handleSave = () => {
    updateMutation.mutate({
      latency_ms: latency || undefined,
      latency_jitter_ms: jitter || undefined,
      packet_loss_rate: lossRate / 100,
      bandwidth_limit_bytes: bandwidth ? bandwidth * 1024 : undefined,
    });
  };

  const isActive = latency > 0 || jitter > 0 || lossRate > 0 || bandwidth > 0;

  if (isLoading) {
    return (
      <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-4">
        <div className="flex items-center justify-center py-4 text-zinc-500">
          <Loader2 size={16} className="animate-spin" />
        </div>
      </div>
    );
  }

  return (
    <div
      className={`rounded-xl border p-4 ${
        isActive
          ? "border-amber-800/50 bg-amber-900/10"
          : "border-zinc-800 bg-zinc-900"
      }`}
    >
      {/* Header — always visible */}
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex w-full items-center gap-2"
      >
        <Wifi
          size={14}
          className={isActive ? "text-amber-400" : "text-zinc-500"}
        />
        <span
          className={`text-sm font-semibold ${
            isActive ? "text-amber-300" : "text-zinc-300"
          }`}
        >
          Network
        </span>

        {/* Active summary chips */}
        {isActive && (
          <div className="flex flex-1 flex-wrap gap-1">
            {latency > 0 && (
              <span className="rounded bg-amber-900/40 px-1.5 py-0.5 text-[10px] font-medium text-amber-300">
                {latency}ms
              </span>
            )}
            {jitter > 0 && (
              <span className="rounded bg-amber-900/40 px-1.5 py-0.5 text-[10px] font-medium text-amber-300">
                ±{jitter}ms
              </span>
            )}
            {lossRate > 0 && (
              <span className="rounded bg-red-900/40 px-1.5 py-0.5 text-[10px] font-medium text-red-300">
                {lossRate}% loss
              </span>
            )}
            {bandwidth > 0 && (
              <span className="rounded bg-amber-900/40 px-1.5 py-0.5 text-[10px] font-medium text-amber-300">
                {bandwidth} KB/s
              </span>
            )}
          </div>
        )}

        {!isActive && (
          <span className="flex-1 text-left text-xs text-zinc-500">
            Off
          </span>
        )}

        <ChevronDown
          size={14}
          className={`shrink-0 text-zinc-500 transition-transform ${
            expanded ? "" : "-rotate-90"
          }`}
        />
      </button>

      {/* Expanded controls */}
      {expanded && (
        <div className="mt-4 space-y-3">
          <SliderField
            id="ns-latency"
            label="Latency"
            description="Fixed delay added to every proxied request. Simulates network round-trip time (e.g. 200ms for a typical mobile connection)."
            value={latency}
            displayValue={`${latency}ms`}
            min={0}
            max={5000}
            step={50}
            onChange={setLatency}
          />
          <SliderField
            id="ns-jitter"
            label="Jitter"
            description="Random variation added on top of latency. Each request gets latency ± a random value up to this amount, simulating unstable connections."
            value={jitter}
            displayValue={`±${jitter}ms`}
            min={0}
            max={2000}
            step={25}
            onChange={setJitter}
          />
          <SliderField
            id="ns-loss"
            label="Packet Loss"
            description="Percentage of requests that will be dropped entirely (no response). Useful for testing retry logic and error handling."
            value={lossRate}
            displayValue={`${lossRate}%`}
            min={0}
            max={100}
            step={1}
            onChange={setLossRate}
          />
          <SliderField
            id="ns-bw"
            label="Bandwidth"
            description="Maximum throughput for response bodies in KB/s. Simulates slow connections like 2G/3G. Set to 0 for unlimited."
            value={bandwidth}
            displayValue={bandwidth ? `${bandwidth} KB/s` : "Unlimited"}
            min={0}
            max={1024}
            step={8}
            onChange={setBandwidth}
          />

          {/* Actions */}
          <div className="flex gap-2 pt-1">
            <button
              type="button"
              onClick={handleSave}
              disabled={updateMutation.isPending}
              className="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-violet-700 disabled:opacity-50"
            >
              <Save size={12} />
              Apply
            </button>
            <button
              type="button"
              onClick={() => clearMutation.mutate()}
              disabled={clearMutation.isPending}
              className="flex items-center gap-1.5 rounded-lg bg-zinc-800 px-3 py-1.5 text-xs text-zinc-400 hover:bg-zinc-700 disabled:opacity-50"
            >
              <Trash2 size={12} />
              Clear
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function SliderField({
  id,
  label,
  description,
  value,
  displayValue,
  min,
  max,
  step,
  onChange,
}: {
  id: string;
  label: string;
  description?: string;
  value: number;
  displayValue: string;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
}) {
  return (
    <div>
      <div className="mb-1 flex justify-between text-xs">
        <span className="flex items-center gap-1 text-zinc-500">
          <label htmlFor={id}>{label}</label>
          {description && <InfoPopover text={description} />}
        </span>
        <span className="font-mono text-zinc-400">{displayValue}</span>
      </div>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseInt(e.target.value))}
        className="w-full accent-violet-500"
      />
    </div>
  );
}

function InfoPopover({ text }: { text: string }) {
  const [open, setOpen] = useState(false);

  return (
    <span className="relative inline-flex">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        className="text-zinc-600 hover:text-zinc-400 transition-colors"
        aria-label="More info"
      >
        <Info size={12} />
      </button>
      {open && (
        <span className="absolute bottom-full left-1/2 z-50 mb-2 w-56 -translate-x-1/2 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-[11px] leading-relaxed text-zinc-300 shadow-lg">
          {text}
          <span className="absolute left-1/2 top-full -translate-x-1/2 border-4 border-transparent border-t-zinc-700" />
        </span>
      )}
    </span>
  );
}
