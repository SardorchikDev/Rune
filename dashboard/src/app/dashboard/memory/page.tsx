"use client";

import { MemoryCard } from "@/components/memory/MemoryCard";
import { MemorySearch } from "@/components/memory/MemorySearch";
import { useMemory } from "@/hooks/useMemory";

/**
 * Semantic memory browser. Lists every entry stored by the reflector,
 * with substring search and per-row delete.
 */
export default function MemoryPage() {
  const { query, setQuery, items, total, loading, error, refresh, remove } =
    useMemory();

  return (
    <div className="h-full overflow-y-auto p-4 space-y-3">
      <MemorySearch
        query={query}
        onQueryChange={setQuery}
        onSubmit={refresh}
        total={total}
        loading={loading}
      />
      {error ? (
        <p className="font-mono text-xs text-accent-red">{error}</p>
      ) : null}
      {items.length === 0 && !loading ? (
        <p className="font-mono text-xs text-muted">
          No memories yet. Memories accumulate as tasks complete with
          reflection enabled.
        </p>
      ) : null}
      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
        {items.map((item) => (
          <MemoryCard key={item.id} item={item} onDelete={remove} />
        ))}
      </div>
    </div>
  );
}
