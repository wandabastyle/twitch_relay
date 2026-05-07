export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export async function safeJson(response: Response): Promise<unknown> {
  try {
    return (await response.json()) as unknown;
  } catch {
    return null;
  }
}

export function readError(payload: unknown): string {
  if (isObject(payload) && typeof payload.error === "string") {
    return payload.error;
  }
  return "request failed";
}

export async function request(input: string, init?: RequestInit): Promise<Response> {
  return fetch(input, {
    credentials: "same-origin",
    ...init,
  });
}
