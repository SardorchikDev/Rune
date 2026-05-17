"use client";

import { useCallback, useEffect, useState } from "react";

import { listTasks } from "./useAgent";
import type { TaskRow } from "@/types/api";

/**
 * Polls `/api/tasks` and keeps a local cache. Refreshes every 6 seconds
 * to pick up streaming status changes from background tasks.
 */
export function useTasks(filter?: { status?: string }) {
  const [tasks, setTasks] = useState<TaskRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      const resp = await listTasks({ status: filter?.status, limit: 200 });
      setTasks(resp.tasks);
    } catch (e) {
      setError(e instanceof Error ? e.message : "failed to load tasks");
    } finally {
      setLoading(false);
    }
  }, [filter?.status]);

  useEffect(() => {
    void refresh();
    const t = setInterval(refresh, 6000);
    return () => clearInterval(t);
  }, [refresh]);

  return { tasks, loading, error, refresh };
}
