/**
 * Shared API response types. These mirror the Rust structs in
 * `backend/src/interfaces/api/*`. Keep in sync manually.
 */

export interface StatusResponse {
  version: string;
  uptime_secs: number;
  active_tasks: number;
  default_provider: string;
  default_model: string;
  cors_origins: string[];
}

export type TaskStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "aborted";

export interface TaskRow {
  id: string;
  session_id: string;
  prompt: string;
  status: TaskStatus;
  provider: string | null;
  model: string | null;
  total_input_tokens: number;
  total_output_tokens: number;
  cost_usd: number;
  started_at: string | null;
  finished_at: string | null;
  created_at: string;
}

export interface AgentLogRow {
  id: number;
  task_id: string;
  iteration: number;
  phase: string;
  content: string;
  metadata: string | null;
  created_at: string;
}

export interface TaskDetail {
  task: TaskRow;
  logs: AgentLogRow[];
}

export interface CreateTaskRequest {
  prompt: string;
  provider?: string | null;
  model?: string | null;
}

export interface CreateTaskResponse {
  task_id: string;
}

export interface ListTasksResponse {
  tasks: TaskRow[];
  total: number;
}

export interface MemoryItem {
  id: string;
  task_id: string | null;
  content: string;
  created_at: string;
}

export interface ListMemoryResponse {
  items: MemoryItem[];
  total: number;
}

export interface ModelResponse {
  provider: string;
  model: string;
  providers: Array<{ name: string; models: string[]; configured: boolean }>;
}

export interface ToolDefinition {
  name: string;
  description: string;
  parameters_schema: Record<string, unknown>;
}

export interface RuneConfigView {
  server: {
    host: string;
    port: number;
    cors_origins: string[];
  };
  telegram: {
    enabled: boolean;
    bot_token: string;
    allowed_user_ids: number[];
  };
  llm: {
    default_provider: string;
    default_model: string;
    stream_tokens: boolean;
    max_retries: number;
    timeout_secs: number;
    failover: { enabled: boolean; order: string[] };
    providers: Record<
      string,
      { api_key: string; base_url: string; models?: string[]; default_model?: string; enabled?: boolean }
    >;
  };
  memory: {
    vector_backend: string;
    qdrant_url: string;
    collection_name: string;
    embedding_provider: string;
    embedding_model: string;
    top_k: number;
  };
  tools: {
    workspace_dir: string;
    terminal_timeout_secs: number;
    allow_web_search: boolean;
    allow_http_fetch: boolean;
    http_fetch_allowlist: string[];
  };
  agent: {
    max_iterations: number;
    system_prompt_path: string;
    reflection_enabled: boolean;
    auto_summarize_threshold: number;
  };
}
