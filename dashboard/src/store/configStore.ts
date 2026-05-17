import { create } from "zustand";

import type { RuneConfigView, StatusResponse } from "@/types/api";

interface ConfigState {
  config: RuneConfigView | null;
  status: StatusResponse | null;
  setConfig(config: RuneConfigView | null): void;
  setStatus(status: StatusResponse | null): void;
}

/**
 * Holds the latest snapshot of `/api/config` and `/api/status` so any
 * panel can read provider/model + connection info without reissuing
 * requests.
 */
export const useConfigStore = create<ConfigState>((set) => ({
  config: null,
  status: null,
  setConfig: (config) => set({ config }),
  setStatus: (status) => set({ status }),
}));
