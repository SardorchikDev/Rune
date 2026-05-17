"use client";

import { useEffect, useState } from "react";

import { apiClient, callApi } from "@/lib/api";
import type { AgentLogRow } from "@/types/api";

/**
 * Raw log viewer. Polls `/api/tasks?limit=10` then fetches each task
 * detail so the dashboard can display the most recent log entries
 * without exposing a dedicated `/api/logs` endpoint.
 */
export default function LogsPage() {
  const [logs, setLogs] = useState<AgentLogRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      try {
        const list = await callApi(() =>
          apiClient
            .get("api/tasks?limit=10")
            .json<{ tasks: { id: string }[] }>()
        );
        const details = await Promise.all(
          list.tasks.map((t) =>
            callApi(() =>
              apiClient.get(`api/tasks/${t.id}`).json<{ logs: AgentLogRow[] }>()
            )
          )
        );
        if (cancelled) return;
        const flat = details
          .flatMap((d) => d.logs)
          .sort(
            (a, b) =>
              Date.parse(b.created_at) - Date.parse(a.created_at)
          )
          .slice(0, 200);
        setLogs(flat);
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : "failed to load logs");
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    void tick();
    const t = setInterval(tick, 8000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);

  return (
    <div className="h-full overflow-y-auto p-4 space-y-2 font-mono text-xs">
      <h2 className="text-[11px] uppercase tracking-widest text-accent-green">
        recent agent logs
      </h2>
      {loading ? <p className="text-muted">loading…</p> : null}
      {error ? <p className="text-accent-red">{error}</p> : null}
      <div className="rune-panel divide-y divide-border">
        {logs.map((l) => (
          <div key={l.id} className="px-3 py-2 space-y-1">
            <div className="flex items-center gap-3 text-[10px] uppercase tracking-widest text-muted">
              <span className="text-accent-cyan">{l.phase}</span>
              <span>task {l.task_id.slice(0, 8)}</span>
              <span>iter {l.iteration}</span>
              <span className="ml-auto">
                {new Date(l.created_at).toLocaleTimeString()}
              </span>
            </div>
            <p className="whitespace-pre-wrap text-primary text-[11px]">
              {l.content}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}
