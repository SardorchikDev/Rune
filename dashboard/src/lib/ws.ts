import type { WsEvent } from "@/types/ws";

import { WS_BASE_URL } from "./api";

/**
 * Backoff schedule for WebSocket reconnect attempts.
 */
export const RECONNECT_BACKOFF_MS = [1000, 2000, 4000, 8000, 16000, 30000];

/**
 * Computes the WebSocket URL used by `useWebSocket`, including the JWT
 * query parameter that the backend's `ws.rs` handler validates.
 */
export function buildWsUrl(token: string): string {
  const base = WS_BASE_URL.replace(/\/+$/, "");
  return `${base}/api/ws?token=${encodeURIComponent(token)}`;
}

export type { WsEvent };
