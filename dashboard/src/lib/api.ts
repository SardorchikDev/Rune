import ky, { HTTPError } from "ky";

import { clearStoredToken, getStoredToken } from "./auth";

/**
 * Base URL for the Rune backend. Falls back to `http://localhost:8080`
 * for `npm run dev` against a locally-running backend.
 */
export const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

/**
 * Base WebSocket URL for the Rune backend.
 */
export const WS_BASE_URL =
  process.env.NEXT_PUBLIC_WS_URL ?? API_BASE_URL.replace(/^http/, "ws");

/**
 * `ky` instance pre-configured with the backend prefix and JWT injection.
 * 401 responses clear the local token and bounce to /login on the next
 * navigation (the middleware handles the actual redirect).
 */
export const apiClient = ky.create({
  prefixUrl: API_BASE_URL,
  timeout: 30000,
  hooks: {
    beforeRequest: [
      (req) => {
        const token = getStoredToken();
        if (token) {
          req.headers.set("Authorization", `Bearer ${token}`);
        }
      },
    ],
    afterResponse: [
      (_req, _opts, res) => {
        if (res.status === 401) {
          clearStoredToken();
        }
        return res;
      },
    ],
  },
});

/**
 * Thin wrapper that re-exports `ky`'s HTTPError as `ApiError` so callers
 * don't have to import from `ky` directly.
 */
export class ApiError extends Error {
  status: number;
  body: unknown;

  constructor(status: number, message: string, body?: unknown) {
    super(message);
    this.status = status;
    this.body = body;
  }

  static async fromKy(err: unknown): Promise<ApiError> {
    if (err instanceof HTTPError) {
      let body: unknown;
      try {
        body = await err.response.clone().json();
      } catch {
        body = undefined;
      }
      return new ApiError(err.response.status, err.message, body);
    }
    if (err instanceof Error) {
      return new ApiError(0, err.message);
    }
    return new ApiError(0, String(err));
  }
}

/**
 * Convenience helper that unwraps HTTPError into ApiError. Use this from
 * mutations / page-level error boundaries.
 */
export async function callApi<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (err) {
    throw await ApiError.fromKy(err);
  }
}
