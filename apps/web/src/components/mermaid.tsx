"use client";

import { useEffect, useRef, useId } from "react";
import mermaid from "mermaid";

mermaid.initialize({
  startOnLoad: false,
  theme: "dark",
  darkMode: true,
  themeVariables: {
    primaryColor: "#8b5cf6",
    primaryTextColor: "#e4e4e7",
    primaryBorderColor: "#3f3f46",
    secondaryColor: "#27272a",
    secondaryTextColor: "#a1a1aa",
    tertiaryColor: "#18181b",
    lineColor: "#71717a",
    textColor: "#d4d4d8",
    mainBkg: "#27272a",
    nodeBorder: "#3f3f46",
    clusterBkg: "#18181b",
    clusterBorder: "#3f3f46",
    titleColor: "#e4e4e7",
    edgeLabelBackground: "#18181b",
    nodeTextColor: "#e4e4e7",
  },
  fontFamily: "ui-monospace, monospace",
  fontSize: 14,
  flowchart: {
    htmlLabels: true,
    curve: "basis",
    padding: 16,
    nodeSpacing: 40,
    rankSpacing: 50,
  },
});

interface MermaidProps {
  chart: string;
  caption?: string;
}

export function Mermaid({ chart, caption }: MermaidProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const id = useId().replace(/:/g, "_");

  useEffect(() => {
    const render = async () => {
      if (!containerRef.current) return;
      try {
        const { svg } = await mermaid.render(`mermaid${id}`, chart.trim());
        containerRef.current.innerHTML = svg;
      } catch {
        containerRef.current.innerHTML = `<pre class="text-red-400 text-sm">${chart}</pre>`;
      }
    };
    render();
  }, [chart, id]);

  return (
    <div className="mermaid-block">
      <div ref={containerRef} className="flex justify-center overflow-x-auto p-6" />
      {caption && (
        <div className="border-t border-zinc-800 px-4 py-2">
          <span className="text-xs text-zinc-500">{caption}</span>
        </div>
      )}
    </div>
  );
}
