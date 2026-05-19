import { isObject, safeJson, readApiError, request } from "./core";
import type { TwitchStatusResponse } from "./types";

export async function getTwitchStatus(): Promise<TwitchStatusResponse> {
  const response = await request("/api/twitch/status");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.connected !== "boolean") {
    throw new Error("twitch status payload is invalid");
  }

  return {
    connected: payload.connected,
    login: typeof payload.login === "string" ? payload.login : undefined,
    display_name: typeof payload.display_name === "string" ? payload.display_name : undefined,
    scopes: Array.isArray(payload.scopes)
      ? payload.scopes.filter((scope): scope is string => typeof scope === "string")
      : [],
  };
}

export function getTwitchConnectUrl(): string {
  return "/api/twitch/connect";
}

export async function disconnectTwitch(): Promise<void> {
  const response = await request("/api/twitch/disconnect", { method: "POST" });
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }
}
