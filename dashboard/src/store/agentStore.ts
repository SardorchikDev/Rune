import { create } from "zustand";

import type { WsEvent } from "@/types/ws";

export type AgentPhase =
  | "idle"
  | "planning"
  | "executing"
  | "reflecting"
  | "complete"
  | "error";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  task_id?: string;
  timestamp: number;
}

export interface ToolCallView {
  call_id: string;
  task_id: string;
  name: string;
  arguments: unknown;
  status: "running" | "complete" | "error";
  output?: string;
  ok?: boolean;
}

interface AgentState {
  connected: boolean;
  currentTaskId: string | null;
  phase: AgentPhase;
  iteration: number;
  messages: ChatMessage[];
  toolCalls: ToolCallView[];
  terminalBuffer: string;
  lastEvent: WsEvent | null;
  setConnected(connected: boolean): void;
  setCurrentTask(task_id: string | null): void;
  appendUserMessage(content: string): void;
  appendEvent(event: WsEvent): void;
  resetBuffer(): void;
  clearTask(): void;
}

const ID = () =>
  typeof crypto !== "undefined" && crypto.randomUUID
    ? crypto.randomUUID()
    : Math.random().toString(36).slice(2);

/**
 * Global agent UI store. Holds the current WebSocket-driven view: phase,
 * iteration counter, streamed terminal buffer, chat history, and tool
 * call cards.
 */
export const useAgentStore = create<AgentState>((set, get) => ({
  connected: false,
  currentTaskId: null,
  phase: "idle",
  iteration: 0,
  messages: [],
  toolCalls: [],
  terminalBuffer: "",
  lastEvent: null,
  setConnected: (connected) => set({ connected }),
  setCurrentTask: (task_id) =>
    set({
      currentTaskId: task_id,
      phase: task_id ? "planning" : "idle",
      iteration: 0,
      terminalBuffer: "",
      toolCalls: [],
    }),
  appendUserMessage: (content) =>
    set((s) => ({
      messages: [
        ...s.messages,
        { id: ID(), role: "user", content, timestamp: Date.now() },
      ],
    })),
  resetBuffer: () => set({ terminalBuffer: "" }),
  clearTask: () =>
    set({
      currentTaskId: null,
      phase: "idle",
      iteration: 0,
      terminalBuffer: "",
      toolCalls: [],
    }),
  appendEvent: (event) => {
    set({ lastEvent: event });
    const current = get().currentTaskId;
    switch (event.type) {
      case "token": {
        if (current && event.task_id !== current) return;
        set((s) => ({
          terminalBuffer: s.terminalBuffer + event.text,
          messages: appendAssistantToken(s.messages, event.task_id, event.text),
        }));
        return;
      }
      case "tool_call": {
        if (current && event.task_id !== current) return;
        set((s) => ({
          phase: "executing",
          toolCalls: [
            ...s.toolCalls,
            {
              call_id: event.call_id,
              task_id: event.task_id,
              name: event.name,
              arguments: event.arguments,
              status: "running",
            },
          ],
        }));
        return;
      }
      case "tool_result": {
        if (current && event.task_id !== current) return;
        set((s) => ({
          toolCalls: s.toolCalls.map((c) =>
            c.call_id === event.call_id
              ? {
                  ...c,
                  status: event.outcome.ok ? "complete" : "error",
                  ok: event.outcome.ok,
                  output: event.outcome.summary,
                }
              : c
          ),
        }));
        return;
      }
      case "status": {
        if (current && event.task_id !== current) return;
        const phase = mapStatusToPhase(event.status);
        set({ phase, iteration: event.iteration });
        return;
      }
      case "final_answer": {
        if (current && event.task_id !== current) return;
        set((s) => ({
          phase: event.status === "completed" ? "complete" : "error",
          messages: replaceAssistantMessage(
            s.messages,
            event.task_id,
            event.text
          ),
          currentTaskId: null,
        }));
        return;
      }
      case "hello":
        return;
    }
  },
}));

function appendAssistantToken(
  messages: ChatMessage[],
  task_id: string,
  delta: string
): ChatMessage[] {
  const last = messages[messages.length - 1];
  if (last && last.role === "assistant" && last.task_id === task_id) {
    const updated: ChatMessage = { ...last, content: last.content + delta };
    return [...messages.slice(0, -1), updated];
  }
  return [
    ...messages,
    { id: ID(), role: "assistant", content: delta, task_id, timestamp: Date.now() },
  ];
}

function replaceAssistantMessage(
  messages: ChatMessage[],
  task_id: string,
  full: string
): ChatMessage[] {
  const last = messages[messages.length - 1];
  if (last && last.role === "assistant" && last.task_id === task_id) {
    const updated: ChatMessage = { ...last, content: full };
    return [...messages.slice(0, -1), updated];
  }
  return [
    ...messages,
    { id: ID(), role: "assistant", content: full, task_id, timestamp: Date.now() },
  ];
}

function mapStatusToPhase(status: string): AgentPhase {
  switch (status) {
    case "planning":
    case "running":
      return "planning";
    case "executing":
      return "executing";
    case "reflecting":
      return "reflecting";
    case "completed":
      return "complete";
    case "failed":
    case "aborted":
      return "error";
    default:
      return "planning";
  }
}
