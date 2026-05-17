"use client";

import { useState } from "react";

import { ApiKeyField } from "@/components/settings/ApiKeyField";
import { ModelSelector } from "@/components/settings/ModelSelector";
import { TelegramWhitelist } from "@/components/settings/TelegramWhitelist";
import { apiClient, callApi } from "@/lib/api";
import { useConfig } from "@/hooks/useConfig";

/**
 * Configuration overview. Displays masked API keys, allows hot-swapping
 * the default model, and exposes toggles for `agent.reflection_enabled`
 * and `agent.max_iterations`.
 */
export default function SettingsPage() {
  const { config, refresh } = useConfig();
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  if (!config) {
    return (
      <p className="p-4 font-mono text-xs text-muted">loading configuration…</p>
    );
  }

  const updateAgent = async (body: Record<string, unknown>) => {
    setPending(true);
    setMessage(null);
    try {
      await callApi(() => apiClient.put("api/config", { json: body }));
      setMessage("saved");
      await refresh();
    } catch (e) {
      setMessage(e instanceof Error ? e.message : "save failed");
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="h-full overflow-y-auto p-4 space-y-4 max-w-4xl">
      <section className="rune-panel p-4 space-y-3">
        <h2 className="font-mono text-[11px] uppercase tracking-widest text-accent-green">
          providers
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
          {Object.entries(config.llm.providers).map(([name, p]) => (
            <ApiKeyField
              key={name}
              label={name}
              value={p.api_key}
              configured={p.api_key.length > 0 || name === "ollama"}
            />
          ))}
        </div>
      </section>

      <ModelSelector config={config} onSaved={refresh} />

      <section className="rune-panel p-4 space-y-3">
        <h2 className="font-mono text-[11px] uppercase tracking-widest text-accent-green">
          telegram
        </h2>
        <p className="font-mono text-[11px] text-muted">
          enabled:{" "}
          <span
            className={
              config.telegram.enabled ? "text-accent-green" : "text-accent-red"
            }
          >
            {String(config.telegram.enabled)}
          </span>
        </p>
        <TelegramWhitelist ids={config.telegram.allowed_user_ids} />
      </section>

      <section className="rune-panel p-4 space-y-3 font-mono text-xs">
        <h2 className="text-[11px] uppercase tracking-widest text-accent-green">
          agent
        </h2>
        <label className="flex items-center justify-between">
          <span className="text-muted">max iterations</span>
          <input
            type="number"
            min={1}
            max={50}
            defaultValue={config.agent.max_iterations}
            onBlur={(e) =>
              updateAgent({ max_iterations: parseInt(e.target.value, 10) })
            }
            className="w-20 bg-bg border border-border rounded px-2 py-1 text-right text-primary"
          />
        </label>
        <label className="flex items-center justify-between">
          <span className="text-muted">reflection enabled</span>
          <input
            type="checkbox"
            defaultChecked={config.agent.reflection_enabled}
            onChange={(e) =>
              updateAgent({ reflection_enabled: e.target.checked })
            }
          />
        </label>
        <p className="text-muted">
          workspace dir: <span className="text-primary">{config.tools.workspace_dir}</span>
        </p>
        {message ? (
          <p
            className={
              message === "saved" ? "text-accent-green" : "text-accent-red"
            }
          >
            {message}
          </p>
        ) : null}
        {pending ? <p className="text-muted">saving…</p> : null}
      </section>

      <section className="rune-panel p-4 space-y-2 font-mono text-xs">
        <h2 className="text-[11px] uppercase tracking-widest text-accent-green">
          memory
        </h2>
        <KeyValue k="backend" v={config.memory.vector_backend} />
        <KeyValue k="collection" v={config.memory.collection_name} />
        <KeyValue
          k="embedding"
          v={`${config.memory.embedding_provider} / ${config.memory.embedding_model}`}
        />
        <KeyValue k="qdrant_url" v={config.memory.qdrant_url} />
        <KeyValue k="top_k" v={String(config.memory.top_k)} />
      </section>
    </div>
  );
}

function KeyValue({ k, v }: { k: string; v: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-muted uppercase tracking-widest text-[10px]">{k}</span>
      <span className="text-primary">{v}</span>
    </div>
  );
}
