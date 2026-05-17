"use client";

import { useAgentStore } from "@/store/agentStore";

/**
 * Slim bottom bar used by the workspace page. Surfaces the current
 * iteration count and tool-call summary without taking up real estate.
 */
export function StatusBar() {
  const phase = useAgentStore((s) => s.phase);
  const toolCalls = useAgentStore((s) => s.toolCalls);
  const running = toolCalls.filter((t) => t.status === "running").length;
  const done = toolCalls.filter((t) => t.status !== "running").length;

  return (
    <div className="h-7 border-t border-border bg-surface px-4 flex items-center gap-4 font-mono text-[11px] text-muted uppercase tracking-widest">
      <span>phase: <span className="text-primary">{phase}</span></span>
      <span>tools running: <span className="text-accent-amber">{running}</span></span>
      <span>tools done: <span className="text-accent-green">{done}</span></span>
    </div>
  );
}
