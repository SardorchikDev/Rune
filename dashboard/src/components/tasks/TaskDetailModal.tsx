"use client";

import { useEffect, useState } from "react";
import { X } from "lucide-react";

import { getTask } from "@/hooks/useAgent";
import type { TaskDetail } from "@/types/api";

/**
 * Modal that loads `/api/tasks/:id` and renders the full prompt, totals,
 * and the timeline of agent_log entries grouped by iteration.
 */
export function TaskDetailModal({
  id,
  onClose,
}: {
  id: string;
  onClose(): void;
}) {
  const [detail, setDetail] = useState<TaskDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const data = await getTask(id);
        if (!cancelled) setDetail(data);
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : "failed to load");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [id]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
      <div className="rune-panel bg-surface w-[min(960px,90vw)] max-h-[85vh] flex flex-col">
        <header className="flex items-center justify-between px-4 h-10 border-b border-border">
          <span className="font-mono text-[11px] uppercase tracking-widest text-accent-green">
            task {id.slice(0, 8)}
          </span>
          <button
            onClick={onClose}
            className="text-muted hover:text-primary"
            aria-label="close"
          >
            <X size={14} />
          </button>
        </header>
        <div className="flex-1 overflow-y-auto p-4 space-y-4 font-mono text-xs">
          {error ? <p className="text-accent-red">{error}</p> : null}
          {!detail && !error ? <p className="text-muted">loading...</p> : null}
          {detail ? (
            <>
              <section className="space-y-1">
                <h3 className="text-[10px] uppercase tracking-widest text-muted">
                  prompt
                </h3>
                <p className="whitespace-pre-wrap text-primary">
                  {detail.task.prompt}
                </p>
              </section>
              <section className="grid grid-cols-2 gap-3 text-[11px]">
                <Stat label="status" value={detail.task.status} />
                <Stat
                  label="provider"
                  value={`${detail.task.provider ?? "—"} / ${detail.task.model ?? "—"}`}
                />
                <Stat
                  label="tokens"
                  value={`${detail.task.total_input_tokens} in · ${detail.task.total_output_tokens} out`}
                />
                <Stat
                  label="cost"
                  value={`$${detail.task.cost_usd.toFixed(4)}`}
                />
              </section>
              <section className="space-y-2">
                <h3 className="text-[10px] uppercase tracking-widest text-muted">
                  timeline ({detail.logs.length})
                </h3>
                {detail.logs.length === 0 ? (
                  <p className="text-muted">no log entries</p>
                ) : (
                  <ol className="space-y-2">
                    {detail.logs.map((l) => (
                      <li key={l.id} className="rune-panel p-3 space-y-1">
                        <div className="flex items-center gap-2 text-[10px] uppercase tracking-widest text-muted">
                          <span className="text-accent-cyan">{l.phase}</span>
                          <span>iter {l.iteration}</span>
                          <span className="ml-auto">
                            {new Date(l.created_at).toLocaleTimeString()}
                          </span>
                        </div>
                        <p className="whitespace-pre-wrap text-primary">
                          {l.content}
                        </p>
                      </li>
                    ))}
                  </ol>
                )}
              </section>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rune-panel p-2">
      <div className="text-[10px] uppercase tracking-widest text-muted">{label}</div>
      <div className="text-primary truncate">{value}</div>
    </div>
  );
}
