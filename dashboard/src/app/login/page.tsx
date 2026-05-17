"use client";

import { Suspense, useState, useTransition } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Terminal } from "lucide-react";

import { setStoredToken } from "@/lib/auth";
import { apiClient, ApiError } from "@/lib/api";

/**
 * Cyberpunk login screen. Sends `{ password }` to `/api/auth/login`,
 * stores the returned JWT, and bounces back to the original `next` route
 * (defaults to `/dashboard/workspace`).
 */
export default function LoginPage() {
  return (
    <Suspense fallback={null}>
      <LoginFormShell />
    </Suspense>
  );
}

function LoginFormShell() {
  const router = useRouter();
  const params = useSearchParams();
  const next = params.get("next") ?? "/dashboard/workspace";
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      const resp = await apiClient
        .post("api/auth/login", { json: { password } })
        .json<{ token: string; expires_at: string }>();
      setStoredToken(resp.token, resp.expires_at);
      startTransition(() => router.push(next));
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) {
        setError("invalid password");
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("login failed");
      }
    }
  };

  return (
    <main className="rune-grid min-h-screen flex items-center justify-center px-6">
      <form
        onSubmit={submit}
        className="rune-panel w-full max-w-sm p-8 space-y-6 font-sans"
      >
        <div className="flex items-center gap-3">
          <Terminal className="text-accent-green" size={20} />
          <span className="font-mono uppercase tracking-widest text-sm rune-glow text-accent-green">
            Rune
          </span>
          <span className="ml-auto text-xs text-muted">v1.0.0</span>
        </div>
        <div className="space-y-2">
          <label className="block text-xs uppercase tracking-widest text-muted">
            dashboard password
          </label>
          <input
            type="password"
            autoFocus
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="w-full bg-bg border border-border rounded px-3 py-2 font-mono text-sm focus:outline-none focus:border-accent-green focus:shadow-glow"
          />
        </div>
        {error ? (
          <p className="text-xs font-mono text-accent-red">{error}</p>
        ) : null}
        <button
          type="submit"
          disabled={pending || password.length === 0}
          className="w-full bg-accent-green/90 text-black uppercase tracking-widest text-sm font-mono py-2 rounded hover:bg-accent-green disabled:opacity-40 disabled:cursor-not-allowed transition"
        >
          {pending ? "..." : "enter"}
        </button>
      </form>
    </main>
  );
}
