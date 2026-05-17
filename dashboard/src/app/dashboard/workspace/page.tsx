"use client";

import { AgentTerminal } from "@/components/workspace/AgentTerminal";
import { ChatPanel } from "@/components/workspace/ChatPanel";
import { ThoughtStream } from "@/components/workspace/ThoughtStream";
import { StatusBar } from "@/components/layout/StatusBar";

/**
 * Three-pane workspace:
 *   - left: live xterm.js terminal streaming agent tokens & tool blocks
 *   - middle: chat panel for issuing prompts & viewing assistant replies
 *   - right: structured tool-call inspector
 */
export default function WorkspacePage() {
  return (
    <div className="flex flex-col h-full">
      <div className="grid grid-cols-12 flex-1 min-h-0">
        <section className="col-span-6 border-r border-border min-h-0">
          <div className="h-full p-2 bg-bg">
            <AgentTerminal />
          </div>
        </section>
        <section className="col-span-4 border-r border-border min-h-0">
          <ChatPanel />
        </section>
        <section className="col-span-2 min-h-0">
          <ThoughtStream />
        </section>
      </div>
      <StatusBar />
    </div>
  );
}
