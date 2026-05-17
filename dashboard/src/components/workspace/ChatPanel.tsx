"use client";

import { useEffect, useRef, useState } from "react";
import { Send, Square } from "lucide-react";

import { useAgent } from "@/hooks/useAgent";
import { useAgentStore } from "@/store/agentStore";
import { useConfigStore } from "@/store/configStore";

/**
 * Right-hand chat panel. Renders the rolling message history and the
 * prompt input. Submitting calls `useAgent.run` which both POSTs the
 * task and seeds the agent store with the user message.
 */
export function ChatPanel() {
  const [prompt, setPrompt] = useState("");
  const messages = useAgentStore((s) => s.messages);
  const currentTaskId = useAgentStore((s) => s.currentTaskId);
  const config = useConfigStore((s) => s.config);
  const { run, abort, pending, error } = useAgent();
  const scrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = prompt.trim();
    if (!trimmed || pending) return;
    try {
      await run({ prompt: trimmed });
      setPrompt("");
    } catch {
      /* useAgent surfaces the error */
    }
  };

  const onKeyDown: React.KeyboardEventHandler<HTMLTextAreaElement> = (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void submit(e as unknown as React.FormEvent);
    }
  };

  return (
    <div className="h-full flex flex-col">
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 space-y-3">
        {messages.length === 0 ? (
          <p className="text-xs font-mono text-muted">
            Submit a prompt to spin up the agent. Live tokens appear in the
            terminal on the left.
          </p>
        ) : (
          messages.map((m) => (
            <div
              key={m.id}
              className="rune-panel p-3 font-mono text-xs space-y-1"
            >
              <div className="flex items-center justify-between text-[10px] uppercase tracking-widest text-muted">
                <span
                  className={
                    m.role === "user"
                      ? "text-accent-cyan"
                      : m.role === "assistant"
                        ? "text-accent-green"
                        : "text-muted"
                  }
                >
                  {m.role}
                </span>
                {m.task_id ? (
                  <span className="text-muted truncate ml-2 max-w-[8rem]">
                    {m.task_id.slice(0, 8)}
                  </span>
                ) : null}
              </div>
              <p className="whitespace-pre-wrap leading-relaxed text-primary">
                {m.content || "…"}
              </p>
            </div>
          ))
        )}
      </div>
      {error ? (
        <p className="px-4 pb-1 text-xs font-mono text-accent-red">{error}</p>
      ) : null}
      <form
        onSubmit={submit}
        className="border-t border-border p-3 space-y-2 bg-surface"
      >
        <div className="flex items-center justify-between text-[10px] uppercase tracking-widest text-muted font-mono">
          <span>
            target: <span className="text-accent-cyan">{config?.llm.default_provider ?? "—"}</span>
            <span className="mx-2 text-muted">/</span>
            <span className="text-primary">{config?.llm.default_model ?? "—"}</span>
          </span>
          {currentTaskId ? (
            <button
              type="button"
              onClick={() => void abort(currentTaskId)}
              className="flex items-center gap-1 text-accent-red hover:text-accent-amber"
            >
              <Square size={11} /> abort
            </button>
          ) : null}
        </div>
        <textarea
          rows={3}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="rune> describe a task..."
          className="w-full resize-none bg-bg border border-border rounded p-2 text-xs font-mono focus:outline-none focus:border-accent-green focus:shadow-glow"
        />
        <div className="flex justify-end">
          <button
            type="submit"
            disabled={pending || prompt.trim().length === 0}
            className="flex items-center gap-2 px-3 py-1.5 bg-accent-green/90 text-black text-[11px] uppercase tracking-widest font-mono rounded hover:bg-accent-green disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Send size={12} />
            {pending ? "submitting" : "run"}
          </button>
        </div>
      </form>
    </div>
  );
}
