export const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null;

interface JsonReadable {
  readonly json: () => Promise<unknown>;
}

export const safeJson = async (response: JsonReadable): Promise<unknown> => {
  try {
    return await response.json();
  } catch {
    return undefined;
  }
};

export const readApiError = (payload: unknown): string => {
  if (isObject(payload) && typeof payload.error === 'string') {
    return payload.error;
  }
  return 'request failed';
};

export const request = async (input: string, ...args: readonly unknown[]): Promise<Response> => {
  const [init] = args;
  const options: RequestInit = { credentials: 'same-origin' };
  if (isObject(init)) {
    Object.assign(options, init as RequestInit);
  }
  const response = await fetch(input, options);
  return response;
};
