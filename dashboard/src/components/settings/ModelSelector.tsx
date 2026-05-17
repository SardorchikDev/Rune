"use client";

import { useState } from "react";

import { apiClient, callApi } from "@/lib/api";
import type { RuneConfigView } from "@/types/api";

interface Props {
  config: RuneConfigView;
  onSaved(): void;
}

/**
 * Settings panel for switching the default LLM provider + model. The
 * PUT happens against `/api/model` which validates the (provider, model)
 * pair against the registered router.
 */
export function ModelSelector({ config, onSaved }: Props) {
  const [provider, setProvider] = useState(config.llm.default_provider);
  const [model, setModel] = useState(config.llm.default_model);
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const providers = Object.keys(config.llm.providers);
  const providerModels = config.llm.providers[provider];
  const suggested =
    providerModels?.models ??
    (providerModels?.default_model ? [providerModels.default_model] : []);

  const save = async () => {
    setPending(true);
    setMessage(null);
    try {
      await callApi(() =>
        apiClient.put("api/model", { json: { provider, model } })
      );
      setMessage("saved");
      onSaved();
    } catch (e) {
      setMessage(e instanceof Error ? e.message : "save failed");
    } finally {
      setPending(false);
    }
  };

  return (
    <section className="rune-panel p-4 space-y-3 font-mono text-xs">
      <h3 className="uppercase tracking-widest text-accent-green text-[11px]">
        default model
      </h3>
      <div className="grid grid-cols-2 gap-3">
        <label className="space-y-1">
          <span className="block text-[10px] uppercase tracking-widest text-muted">
            provider
          </span>
          <select
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
            className="w-full bg-bg border border-border rounded px-2 py-1 text-primary"
          >
            {providers.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </label>
        <label className="space-y-1">
          <span className="block text-[10px] uppercase tracking-widest text-muted">
            model
          </span>
          <input
            value={model}
            onChange={(e) => setModel(e.target.value)}
            list={`models-${provider}`}
            className="w-full bg-bg border border-border rounded px-2 py-1 text-primary"
          />
          <datalist id={`models-${provider}`}>
            {suggested.map((m) => (
              <option key={m} value={m} />
            ))}
          </datalist>
        </label>
      </div>
      <div className="flex items-center gap-3">
        <button
          onClick={save}
          disabled={pending}
          className="px-3 py-1 bg-accent-green/90 text-black uppercase tracking-widest text-[11px] rounded hover:bg-accent-green disabled:opacity-40"
        >
          {pending ? "saving…" : "save"}
        </button>
        {message ? (
          <span
            className={
              message === "saved" ? "text-accent-green" : "text-accent-red"
            }
          >
            {message}
          </span>
        ) : null}
      </div>
    </section>
  );
}
