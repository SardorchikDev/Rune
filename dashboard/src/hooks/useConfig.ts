"use client";

import { useCallback, useEffect } from "react";

import { apiClient, callApi } from "@/lib/api";
import { useConfigStore } from "@/store/configStore";
import type { RuneConfigView, StatusResponse } from "@/types/api";

/**
 * Loads `/api/status` + `/api/config` and refreshes them every 10s.
 * The first paint happens on mount; subsequent polls live for the
 * lifetime of the consumer hook.
 */
export function useConfig() {
  const setConfig = useConfigStore((s) => s.setConfig);
  const setStatus = useConfigStore((s) => s.setStatus);
  const status = useConfigStore((s) => s.status);
  const config = useConfigStore((s) => s.config);

  const refresh = useCallback(async () => {
    try {
      const [statusResp, configResp] = await Promise.all([
        callApi(() => apiClient.get("api/status").json<StatusResponse>()),
        callApi(() => apiClient.get("api/config").json<RuneConfigView>()),
      ]);
      setStatus(statusResp);
      setConfig(configResp);
    } catch {
      // swallow — `/api/config` is gated behind auth and may legitimately fail
    }
  }, [setConfig, setStatus]);

  useEffect(() => {
    void refresh();
    const t = setInterval(refresh, 10000);
    return () => clearInterval(t);
  }, [refresh]);

  return { config, status, refresh };
}
