/**
 * WebSocket events emitted by the Rune backend. Mirrors the
 * `WsEvent` enum in `backend/src/interfaces/api/ws.rs`.
 */

export type WsEvent =
  | { type: "hello"; uptime_secs: number }
  | { type: "token"; task_id: string; text: string }
  | { type: "tool_call"; task_id: string; name: string; arguments: unknown; call_id: string }
  | { type: "tool_result"; task_id: string; call_id: string; name: string; outcome: ToolOutcomeView }
  | { type: "status"; task_id: string; status: string; iteration: number }
  | { type: "final_answer"; task_id: string; text: string; status: string };

export interface ToolOutcomeView {
  ok: boolean;
  summary: string;
  details: unknown;
}
