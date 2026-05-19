import { isObject, safeJson, readApiError, request } from "./core";
import type { WatchSessionResponse } from "./types";

export async function getWatchSession(
  ticket: string,
  relay = false,
): Promise<WatchSessionResponse> {
  const query = relay ? "?relay=1" : "";
  const response = await request(`/api/watch-session/${encodeURIComponent(ticket)}${query}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    typeof payload.channel !== "string" ||
    typeof payload.manifest_url !== "string" ||
    typeof payload.relay !== "boolean" ||
    typeof payload.app_version !== "string"
  ) {
    throw new Error("watch session payload is invalid");
  }

  return {
    channel: payload.channel,
    manifest_url: payload.manifest_url,
    relay: payload.relay,
    app_version: payload.app_version,
  };
}
