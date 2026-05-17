"use client";

import { Search } from "lucide-react";

interface Props {
  query: string;
  onQueryChange(value: string): void;
  onSubmit(): void;
  total: number;
  loading: boolean;
}

/**
 * Memory browser search bar. Substring search (not vector search) — the
 * backend filters from the SQLite index using LIKE.
 */
export function MemorySearch({
  query,
  onQueryChange,
  onSubmit,
  total,
  loading,
}: Props) {
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
      className="flex items-center gap-2 rune-panel p-2 font-mono text-xs"
    >
      <Search size={14} className="text-muted" />
      <input
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
        placeholder="search memory..."
        className="flex-1 bg-transparent focus:outline-none text-primary"
      />
      <button
        type="submit"
        className="px-3 py-1 bg-accent-green/90 text-black uppercase tracking-widest text-[11px] rounded hover:bg-accent-green"
      >
        search
      </button>
      <span className="text-muted">
        {loading ? "…" : `${total} match(es)`}
      </span>
    </form>
  );
}
