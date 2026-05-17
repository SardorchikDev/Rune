"use client";

import { CheckCircle2, Loader2, XCircle } from "lucide-react";

import type { ToolCallView } from "@/store/agentStore";

/**
 * Compact card describing a single tool invocation. Renders in the side
 * column of the workspace beneath the chat.
 */
export function ToolCallCard({ call }: { call: ToolCallView }) {
  return (
    <div className="rune-panel p-3 space-y-2 font-mono text-[11px]">
      <div className="flex items-center justify-between text-[10px] uppercase tracking-widest">
        <span className="text-accent-amber">{call.name}</span>
        <span className="text-muted truncate ml-2 max-w-[10rem]">
          {call.call_id.slice(-8)}
        </span>
        <StatusIcon status={call.status} ok={call.ok} />
      </div>
      <pre className="text-muted whitespace-pre-wrap break-words text-[10px]">
        {typeof call.arguments === "string"
          ? call.arguments
          : JSON.stringify(call.arguments, null, 2)}
      </pre>
      {call.output ? (
        <p
          className={`whitespace-pre-wrap text-[11px] ${
            call.ok === false ? "text-accent-red" : "text-primary"
          }`}
        >
          {call.output}
        </p>
      ) : null}
    </div>
  );
}

function StatusIcon({
  status,
  ok,
}: {
  status: ToolCallView["status"];
  ok?: boolean;
}) {
  if (status === "running") {
    return <Loader2 size={12} className="animate-spin text-accent-amber" />;
  }
  if (ok === false || status === "error") {
    return <XCircle size={12} className="text-accent-red" />;
  }
  return <CheckCircle2 size={12} className="text-accent-green" />;
}
