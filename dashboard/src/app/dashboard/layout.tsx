"use client";

import type { ReactNode } from "react";

import { Sidebar } from "@/components/layout/Sidebar";
import { Topbar } from "@/components/layout/Topbar";
import { useConfig } from "@/hooks/useConfig";
import { useWebSocket } from "@/hooks/useWebSocket";

/**
 * Persistent shell for every `/dashboard/*` page. Boots the WebSocket
 * subscription and the `/api/status` poll on first mount.
 */
export default function DashboardLayout({ children }: { children: ReactNode }) {
  useWebSocket();
  useConfig();

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <Topbar />
        <main className="flex-1 min-h-0 overflow-hidden">{children}</main>
      </div>
    </div>
  );
}
