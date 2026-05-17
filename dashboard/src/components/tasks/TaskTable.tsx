"use client";

import { useState } from "react";

import { TaskDetailModal } from "./TaskDetailModal";
import type { TaskRow } from "@/types/api";

const STATUS_COLOR: Record<string, string> = {
  pending: "text-muted",
  running: "text-accent-amber",
  completed: "text-accent-green",
  failed: "text-accent-red",
  aborted: "text-muted",
};

/**
 * Sortable, click-to-inspect task history table. Click any row to open
 * the detail modal with the full agent log timeline.
 */
export function TaskTable({ tasks }: { tasks: TaskRow[] }) {
  const [selected, setSelected] = useState<string | null>(null);

  return (
    <>
      <div className="overflow-auto rune-panel">
        <table className="min-w-full font-mono text-xs">
          <thead className="bg-bg sticky top-0">
            <tr className="text-left text-[10px] uppercase tracking-widest text-muted">
              <Th>id</Th>
              <Th>status</Th>
              <Th>prompt</Th>
              <Th>provider</Th>
              <Th>tokens</Th>
              <Th>cost</Th>
              <Th>started</Th>
            </tr>
          </thead>
          <tbody>
            {tasks.length === 0 ? (
              <tr>
                <td colSpan={7} className="p-6 text-center text-muted">
                  no tasks yet
                </td>
              </tr>
            ) : (
              tasks.map((t) => (
                <tr
                  key={t.id}
                  onClick={() => setSelected(t.id)}
                  className="border-t border-border hover:bg-bg cursor-pointer"
                >
                  <Td className="text-primary">{t.id.slice(0, 8)}</Td>
                  <Td className={STATUS_COLOR[t.status] ?? "text-primary"}>
                    {t.status}
                  </Td>
                  <Td className="max-w-md truncate text-primary">{t.prompt}</Td>
                  <Td className="text-muted">
                    {t.provider ?? "—"}
                    <span className="mx-1">/</span>
                    {t.model ?? "—"}
                  </Td>
                  <Td className="text-muted">
                    {t.total_input_tokens}
                    <span className="mx-1">/</span>
                    {t.total_output_tokens}
                  </Td>
                  <Td className="text-accent-amber">
                    ${t.cost_usd.toFixed(4)}
                  </Td>
                  <Td className="text-muted">
                    {t.started_at
                      ? new Date(t.started_at).toLocaleString()
                      : "—"}
                  </Td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      {selected ? (
        <TaskDetailModal id={selected} onClose={() => setSelected(null)} />
      ) : null}
    </>
  );
}

function Th({ children }: { children: React.ReactNode }) {
  return <th className="px-3 py-2">{children}</th>;
}
function Td({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return <td className={`px-3 py-2 ${className ?? ""}`}>{children}</td>;
}
