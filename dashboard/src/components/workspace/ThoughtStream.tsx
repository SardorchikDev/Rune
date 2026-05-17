"use client";

import { useAgentStore } from "@/store/agentStore";
import { ToolCallCard } from "./ToolCallCard";

/**
 * Right-hand inspector panel that lists live tool calls in chronological
 * order. Mirrors what the terminal shows but with structured cards so
 * inputs/outputs can be re-read easily.
 */
export function ThoughtStream() {
  const toolCalls = useAgentStore((s) => s.toolCalls);

  return (
    <div className="h-full overflow-y-auto p-3 space-y-2 bg-bg">
      <h3 className="text-[10px] uppercase tracking-widest text-muted font-mono">
        tool calls
      </h3>
      {toolCalls.length === 0 ? (
        <p className="text-[11px] font-mono text-muted">
          No tool activity yet.
        </p>
      ) : (
        toolCalls
          .slice()
          .reverse()
          .map((c) => <ToolCallCard key={c.call_id} call={c} />)
      )}
    </div>
  );
}
