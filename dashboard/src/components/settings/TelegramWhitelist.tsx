"use client";

import { X } from "lucide-react";

interface Props {
  ids: number[];
}

/**
 * Read-only display of the Telegram whitelist. Writes are not supported
 * over the API — operators edit `config.toml` directly to add/remove ids
 * (this is part of the security model in section 10).
 */
export function TelegramWhitelist({ ids }: Props) {
  return (
    <div className="space-y-1">
      <span className="block text-[10px] uppercase tracking-widest text-muted">
        whitelist
      </span>
      <div className="flex flex-wrap gap-2">
        {ids.length === 0 ? (
          <span className="text-muted text-[11px]">no users authorised</span>
        ) : (
          ids.map((id) => (
            <span
              key={id}
              className="inline-flex items-center gap-1 px-2 py-0.5 border border-border rounded text-[11px]"
            >
              <span className="text-accent-cyan">{id}</span>
              <X size={10} className="text-muted/40" />
            </span>
          ))
        )}
      </div>
      <p className="text-[10px] text-muted">
        Edit `config.toml` to update this list.
      </p>
    </div>
  );
}
