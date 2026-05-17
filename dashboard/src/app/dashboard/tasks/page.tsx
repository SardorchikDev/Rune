"use client";

import { useState } from "react";

import { TaskTable } from "@/components/tasks/TaskTable";
import { useTasks } from "@/hooks/useTasks";

const STATUSES = ["", "pending", "running", "completed", "failed", "aborted"];

/**
 * Task history page. Renders a filterable table with a click-through
 * detail modal.
 */
export default function TasksPage() {
  const [status, setStatus] = useState("");
  const { tasks, loading, error } = useTasks({ status: status || undefined });

  return (
    <div className="h-full overflow-y-auto p-4 space-y-3">
      <header className="flex items-center gap-3 font-mono text-[11px] uppercase tracking-widest">
        <span className="text-muted">filter</span>
        {STATUSES.map((s) => (
          <button
            key={s || "all"}
            onClick={() => setStatus(s)}
            className={`px-2 py-1 rounded border ${
              status === s
                ? "border-accent-green text-accent-green shadow-glow"
                : "border-border text-muted hover:text-primary"
            }`}
          >
            {s || "all"}
          </button>
        ))}
        <span className="ml-auto text-muted">
          {loading ? "loading…" : `${tasks.length} task(s)`}
        </span>
      </header>
      {error ? (
        <p className="font-mono text-xs text-accent-red">{error}</p>
      ) : null}
      <TaskTable tasks={tasks} />
    </div>
  );
}
