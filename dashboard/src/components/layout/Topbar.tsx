"use client";

import { useRouter } from "next/navigation";
import { LogOut, RefreshCcw } from "lucide-react";

import { clearStoredToken } from "@/lib/auth";
import { useAgentStore, type AgentPhase } from "@/store/agentStore";
import { useConfigStore } from "@/store/configStore";

const PHASE_LABEL: Record<AgentPhase, string> = {
  idle: "idle",
  planning: "planning",
  executing: "executing",
  reflecting: "reflecting",
  complete: "complete",
  error: "error",
};

const PHASE_COLOR: Record<AgentPhase, string> = {
  idle: "text-muted",
  planning: "text-accent-cyan",
  executing: "text-accent-amber",
  reflecting: "text-accent-green",
  complete: "text-accent-green",
  error: "text-accent-red",
};

/**
 * Top bar showing the current agent phase, current task id, and a
 * compact logout button.
 */
export function Topbar() {
  const router = useRouter();
  const phase = useAgentStore((s) => s.phase);
  const taskId = useAgentStore((s) => s.currentTaskId);
  const iteration = useAgentStore((s) => s.iteration);
  const status = useConfigStore((s) => s.status);

  const logout = () => {
    clearStoredToken();
    router.push("/login");
  };

  return (
    <header className="h-12 border-b border-border bg-surface px-4 flex items-center gap-4 font-mono text-xs">
      <span className="uppercase tracking-widest text-muted">phase</span>
      <span className={`${PHASE_COLOR[phase]} rune-glow uppercase`}>{PHASE_LABEL[phase]}</span>
      <span className="text-muted">|</span>
      <span className="text-muted uppercase tracking-widest">task</span>
      <span className="text-primary truncate max-w-[12rem]">
        {taskId ? taskId : "—"}
      </span>
      <span className="text-muted">|</span>
      <span className="text-muted uppercase tracking-widest">iter</span>
      <span className="text-accent-amber">{iteration}</span>
      <div className="ml-auto flex items-center gap-3">
        <span className="text-muted">
          uptime <span className="text-primary">{status ? formatUptime(status.uptime_secs) : "—"}</span>
        </span>
        <button
          type="button"
          onClick={() => location.reload()}
          className="text-muted hover:text-primary"
          aria-label="refresh"
        >
          <RefreshCcw size={14} />
        </button>
        <button
          type="button"
          onClick={logout}
          className="text-muted hover:text-accent-red uppercase tracking-widest text-[11px] flex items-center gap-1"
        >
          <LogOut size={14} />
          logout
        </button>
      </div>
    </header>
  );
}

function formatUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return `${h}h ${m}m`;
}
