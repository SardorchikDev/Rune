/**
 * JWT lifecycle helpers shared between the API client and the page tree.
 * Mirrors the cookie the middleware checks, plus a localStorage copy that
 * the WebSocket connection uses (cookies don't get sent on `ws://`).
 */

export const TOKEN_STORAGE_KEY = "rune_jwt";
export const TOKEN_EXPIRES_KEY = "rune_jwt_expires";
export const TOKEN_COOKIE_NAME = "rune_jwt";

export function getStoredToken(): string | null {
  if (typeof window === "undefined") return null;
  const token = window.localStorage.getItem(TOKEN_STORAGE_KEY);
  if (!token) return null;
  const expires = window.localStorage.getItem(TOKEN_EXPIRES_KEY);
  if (expires && Date.parse(expires) < Date.now()) {
    clearStoredToken();
    return null;
  }
  return token;
}

export function setStoredToken(token: string, expiresAt: string): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(TOKEN_STORAGE_KEY, token);
  window.localStorage.setItem(TOKEN_EXPIRES_KEY, expiresAt);
  const maxAge = Math.max(1, Math.floor((Date.parse(expiresAt) - Date.now()) / 1000));
  document.cookie = `${TOKEN_COOKIE_NAME}=${token}; path=/; max-age=${maxAge}; SameSite=Lax`;
}

export function clearStoredToken(): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(TOKEN_STORAGE_KEY);
  window.localStorage.removeItem(TOKEN_EXPIRES_KEY);
  document.cookie = `${TOKEN_COOKIE_NAME}=; path=/; max-age=0; SameSite=Lax`;
}
