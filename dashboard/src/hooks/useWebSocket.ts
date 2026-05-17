"use client";

import { useEffect, useRef } from "react";

import { getStoredToken } from "@/lib/auth";
import { RECONNECT_BACKOFF_MS, buildWsUrl } from "@/lib/ws";
import { useAgentStore } from "@/store/agentStore";
import type { WsEvent } from "@/types/ws";

/**
 * Subscribes the global agent store to the backend's WebSocket bus and
 * reconnects on failure with exponential backoff (1s, 2s, 4s, 8s, max 30s).
 * Returns the current `connected` state for convenience.
 */
export function useWebSocket(): { connected: boolean } {
  const setConnected = useAgentStore((s) => s.setConnected);
  const appendEvent = useAgentStore((s) => s.appendEvent);
  const connected = useAgentStore((s) => s.connected);
  const attemptRef = useRef(0);
  const closedRef = useRef(false);

  useEffect(() => {
    closedRef.current = false;
    let ws: WebSocket | null = null;
    let timeout: ReturnType<typeof setTimeout> | null = null;

    const open = () => {
      const token = getStoredToken();
      if (!token) {
        attemptRef.current += 1;
        scheduleReconnect();
        return;
      }
      try {
        ws = new WebSocket(buildWsUrl(token));
      } catch {
        scheduleReconnect();
        return;
      }
      ws.onopen = () => {
        attemptRef.current = 0;
        setConnected(true);
      };
      ws.onmessage = (msg) => {
        try {
          const event = JSON.parse(msg.data) as WsEvent;
          appendEvent(event);
        } catch {
          /* ignore malformed frame */
        }
      };
      ws.onerror = () => {
        setConnected(false);
      };
      ws.onclose = () => {
        setConnected(false);
        if (!closedRef.current) scheduleReconnect();
      };
    };

    const scheduleReconnect = () => {
      if (closedRef.current) return;
      const delay =
        RECONNECT_BACKOFF_MS[Math.min(attemptRef.current, RECONNECT_BACKOFF_MS.length - 1)];
      attemptRef.current += 1;
      timeout = setTimeout(open, delay);
    };

    open();

    return () => {
      closedRef.current = true;
      if (timeout) clearTimeout(timeout);
      if (ws) ws.close();
      setConnected(false);
    };
  }, [appendEvent, setConnected]);

  return { connected };
}
