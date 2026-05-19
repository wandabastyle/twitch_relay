import { isObject, safeJson, readApiError, request } from "./core";
import type { VersionResponse } from "./types";

export async function getVersion(): Promise<VersionResponse> {
  const response = await request("/api/version");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.version !== "string") {
    throw new Error("version payload is invalid");
  }

  return { version: payload.version };
}
