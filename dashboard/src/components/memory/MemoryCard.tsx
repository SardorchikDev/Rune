"use client";

import { Trash2 } from "lucide-react";

import type { MemoryItem } from "@/types/api";

interface Props {
  item: MemoryItem;
  onDelete(id: string): void;
}

/**
 * Single semantic memory entry. Click delete to remove from the index +
 * Qdrant collection.
 */
export function MemoryCard({ item, onDelete }: Props) {
  const created = new Date(item.created_at);
  return (
    <article className="rune-panel p-3 font-mono text-xs space-y-2">
      <header className="flex items-center justify-between text-[10px] uppercase tracking-widest text-muted">
        <span>{relative(created)}</span>
        {item.task_id ? (
          <span className="text-muted">
            task <span className="text-accent-cyan">{item.task_id.slice(0, 8)}</span>
          </span>
        ) : null}
        <button
          onClick={() => onDelete(item.id)}
          className="text-muted hover:text-accent-red"
          aria-label="delete"
        >
          <Trash2 size={12} />
        </button>
      </header>
      <p className="whitespace-pre-wrap text-primary text-[11px]">
        {item.content}
      </p>
    </article>
  );
}

function relative(d: Date): string {
  const diff = (Date.now() - d.getTime()) / 1000;
  if (diff < 60) return `${Math.floor(diff)}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}
