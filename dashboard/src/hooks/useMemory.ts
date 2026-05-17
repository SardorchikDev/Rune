"use client";

import { useCallback, useEffect, useState } from "react";

import { apiClient, callApi } from "@/lib/api";
import type { ListMemoryResponse, MemoryItem } from "@/types/api";

/**
 * Memory browser fetcher. Supports substring search via `query`.
 */
export function useMemory(initialQuery = "") {
  const [query, setQuery] = useState(initialQuery);
  const [items, setItems] = useState<MemoryItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const search = new URLSearchParams();
      if (query) search.set("query", query);
      search.set("limit", "50");
      const resp = await callApi(() =>
        apiClient
          .get(`api/memory?${search.toString()}`)
          .json<ListMemoryResponse>()
      );
      setItems(resp.items);
      setTotal(resp.total);
    } catch (e) {
      setError(e instanceof Error ? e.message : "failed to load memory");
    } finally {
      setLoading(false);
    }
  }, [query]);

  const remove = useCallback(
    async (id: string) => {
      await callApi(() => apiClient.delete(`api/memory/${id}`));
      await refresh();
    },
    [refresh]
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { query, setQuery, items, total, loading, error, refresh, remove };
}
