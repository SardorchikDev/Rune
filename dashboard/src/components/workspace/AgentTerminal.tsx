"use client";

import { useEffect, useRef } from "react";

import { useAgentStore } from "@/store/agentStore";

type XTermType = typeof import("@xterm/xterm");
type FitAddonType = typeof import("@xterm/addon-fit");

/**
 * Lightweight xterm.js wrapper. Loads xterm dynamically so SSR is happy,
 * mounts a single Terminal into the provided container, and streams the
 * `terminalBuffer` slice from the agent store as new bytes arrive.
 */
export function AgentTerminal() {
  const ref = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<import("@xterm/xterm").Terminal | null>(null);
  const fitRef = useRef<import("@xterm/addon-fit").FitAddon | null>(null);
  const writtenRef = useRef(0);
  const buffer = useAgentStore((s) => s.terminalBuffer);
  const toolCalls = useAgentStore((s) => s.toolCalls);
  const lastToolIdRef = useRef<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let resizeObserver: ResizeObserver | null = null;

    void (async () => {
      const xtermMod: XTermType = await import("@xterm/xterm");
      const fitMod: FitAddonType = await import("@xterm/addon-fit");
      // @ts-expect-error - side-effect CSS import has no type declaration
      await import("@xterm/xterm/css/xterm.css");
      if (disposed || !ref.current) return;

      const term = new xtermMod.Terminal({
        fontFamily: "JetBrains Mono, ui-monospace, monospace",
        fontSize: 12,
        theme: {
          background: "#080b0f",
          foreground: "#e6edf3",
          cursor: "#00ff88",
          green: "#00ff88",
          red: "#ff3b30",
          yellow: "#ffbb33",
          blue: "#5be8ff",
          brightBlack: "#7d8590",
        },
        cursorBlink: true,
        convertEol: true,
        scrollback: 4000,
      });
      const fit = new fitMod.FitAddon();
      term.loadAddon(fit);
      term.open(ref.current);
      fit.fit();
      term.writeln("\x1b[2mRune workspace ready. Awaiting agent run.\x1b[0m");
      termRef.current = term;
      fitRef.current = fit;

      resizeObserver = new ResizeObserver(() => {
        try {
          fit.fit();
        } catch {
          /* layout race — ignore */
        }
      });
      if (ref.current) resizeObserver.observe(ref.current);
    })();

    return () => {
      disposed = true;
      resizeObserver?.disconnect();
      termRef.current?.dispose();
      termRef.current = null;
    };
  }, []);

  // Stream the agent buffer in incrementally rather than rewriting on every
  // tick. We track how many bytes we've already written.
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;
    if (writtenRef.current > buffer.length) {
      // buffer was reset; clear screen
      term.reset();
      writtenRef.current = 0;
    }
    const delta = buffer.slice(writtenRef.current);
    if (delta.length > 0) {
      term.write(delta);
      writtenRef.current = buffer.length;
    }
  }, [buffer]);

  // Emit colored headers when a new tool call is dispatched.
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;
    const last = toolCalls[toolCalls.length - 1];
    if (!last) return;
    if (last.call_id === lastToolIdRef.current) return;
    lastToolIdRef.current = last.call_id;
    term.writeln("");
    term.writeln(`\x1b[33m[tool ${last.name}]\x1b[0m`);
    const args =
      typeof last.arguments === "object"
        ? JSON.stringify(last.arguments)
        : String(last.arguments);
    term.writeln(`\x1b[2m${args}\x1b[0m`);
  }, [toolCalls]);

  return <div ref={ref} className="h-full w-full" />;
}
