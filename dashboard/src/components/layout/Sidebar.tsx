"use client";

import { usePathname } from "next/navigation";
import Link from "next/link";
import {
  Activity,
  Brain,
  Cpu,
  History,
  Settings as SettingsIcon,
  Terminal,
} from "lucide-react";

import { useAgentStore } from "@/store/agentStore";
import { useConfigStore } from "@/store/configStore";

const LINKS = [
  { href: "/dashboard/workspace", label: "Workspace", icon: Terminal },
  { href: "/dashboard/tasks", label: "Tasks", icon: History },
  { href: "/dashboard/memory", label: "Memory", icon: Brain },
  { href: "/dashboard/logs", label: "Logs", icon: Activity },
  { href: "/dashboard/settings", label: "Settings", icon: SettingsIcon },
];

/**
 * Left navigation rail. Renders the Rune wordmark, primary nav, and a
 * pinned status block showing the active provider and live connection
 * indicator.
 */
export function Sidebar() {
  const pathname = usePathname();
  const status = useConfigStore((s) => s.status);
  const connected = useAgentStore((s) => s.connected);
  const provider = status?.default_provider ?? "—";
  const model = status?.default_model ?? "—";
  const active = status?.active_tasks ?? 0;

  return (
    <aside className="w-60 shrink-0 border-r border-border bg-surface flex flex-col">
      <div className="h-12 px-4 flex items-center gap-2 border-b border-border">
        <Cpu className="text-accent-green" size={18} />
        <span className="font-mono text-sm uppercase tracking-widest rune-glow text-accent-green">
          Rune
        </span>
        <span className="ml-auto text-[10px] font-mono text-muted">v1.0.0</span>
      </div>
      <nav className="flex-1 p-3 space-y-1">
        {LINKS.map((link) => {
          const Icon = link.icon;
          const active = pathname === link.href || pathname.startsWith(link.href + "/");
          return (
            <Link
              key={link.href}
              href={link.href}
              className={`flex items-center gap-3 px-3 py-2 rounded font-mono text-xs uppercase tracking-widest ${
                active
                  ? "bg-bg text-accent-green shadow-glow"
                  : "text-muted hover:text-primary hover:bg-bg"
              }`}
            >
              <Icon size={14} />
              {link.label}
            </Link>
          );
        })}
      </nav>
      <div className="p-3 border-t border-border space-y-2">
        <div className="rune-panel p-3 space-y-2 font-mono text-[11px]">
          <div className="flex items-center justify-between">
            <span className="text-muted">provider</span>
            <span className="text-accent-cyan">{provider}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted">model</span>
            <span className="text-primary truncate ml-3">{model}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted">tasks</span>
            <span className="text-accent-amber">{active}</span>
          </div>
          <div className="flex items-center gap-2">
            <span
              className={`inline-block w-2 h-2 rounded-full ${
                connected ? "bg-accent-green shadow-glow" : "bg-accent-red"
              }`}
            />
            <span className="text-muted">{connected ? "connected" : "offline"}</span>
          </div>
        </div>
      </div>
    </aside>
  );
}
