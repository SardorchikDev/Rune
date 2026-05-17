"use client";

import { useCallback, useState } from "react";

import { apiClient, callApi } from "@/lib/api";
import { useAgentStore } from "@/store/agentStore";
import type {
  CreateTaskRequest,
  CreateTaskResponse,
  TaskDetail,
  TaskRow,
} from "@/types/api";

/**
 * Mutations for kicking off / aborting / inspecting agent tasks.
 */
export function useAgent() {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const setCurrentTask = useAgentStore((s) => s.setCurrentTask);
  const appendUserMessage = useAgentStore((s) => s.appendUserMessage);
  const clearTask = useAgentStore((s) => s.clearTask);

  const run = useCallback(
    async (body: CreateTaskRequest) => {
      setPending(true);
      setError(null);
      try {
        const resp = await callApi(() =>
          apiClient.post("api/tasks", { json: body }).json<CreateTaskResponse>()
        );
        appendUserMessage(body.prompt);
        setCurrentTask(resp.task_id);
        return resp;
      } catch (e) {
        setError(e instanceof Error ? e.message : "failed to start task");
        throw e;
      } finally {
        setPending(false);
      }
    },
    [appendUserMessage, setCurrentTask]
  );

  const abort = useCallback(
    async (task_id: string) => {
      try {
        await callApi(() =>
          apiClient.post("api/agent/abort", { json: { task_id } })
        );
        clearTask();
      } catch (e) {
        setError(e instanceof Error ? e.message : "failed to abort task");
        throw e;
      }
    },
    [clearTask]
  );

  return { run, abort, pending, error };
}

/**
 * Read-only helpers for the tasks page.
 */
export async function listTasks(params?: { status?: string; limit?: number }) {
  const search = new URLSearchParams();
  if (params?.status) search.set("status", params.status);
  if (params?.limit) search.set("limit", String(params.limit));
  const qs = search.toString();
  return callApi(() =>
    apiClient
      .get(`api/tasks${qs ? `?${qs}` : ""}`)
      .json<{ tasks: TaskRow[]; total: number }>()
  );
}

export async function getTask(id: string) {
  return callApi(() => apiClient.get(`api/tasks/${id}`).json<TaskDetail>());
}
