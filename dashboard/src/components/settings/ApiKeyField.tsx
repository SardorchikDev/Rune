"use client";

import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";

interface Props {
  label: string;
  value: string;
  configured: boolean;
}

/**
 * Read-only display for a masked API key. The backend masks every key
 * with `sk-****…` so this field never receives the raw secret over the
 * wire — toggling the eye just reveals the masked string.
 */
export function ApiKeyField({ label, value, configured }: Props) {
  const [show, setShow] = useState(false);
  return (
    <div className="rune-panel p-3 space-y-2 font-mono text-xs">
      <div className="flex items-center justify-between text-[10px] uppercase tracking-widest">
        <span className="text-muted">{label}</span>
        <span
          className={configured ? "text-accent-green" : "text-muted"}
          aria-label={configured ? "configured" : "missing"}
        >
          {configured ? "configured" : "missing"}
        </span>
      </div>
      <div className="flex items-center gap-2">
        <input
          readOnly
          type={show ? "text" : "password"}
          value={value || ""}
          className="flex-1 bg-bg border border-border rounded px-2 py-1 text-[11px] text-primary"
        />
        <button
          type="button"
          onClick={() => setShow((v) => !v)}
          className="text-muted hover:text-primary"
          aria-label={show ? "hide" : "show"}
        >
          {show ? <EyeOff size={14} /> : <Eye size={14} />}
        </button>
      </div>
    </div>
  );
}
