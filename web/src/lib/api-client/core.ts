export const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null;

export const safeJson = async (response: Response): Promise<unknown> => {
  try {
    return (await response.json()) as unknown;
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

export const request = async (input: string, init?: RequestInit): Promise<Response> =>
  fetch(input, {
    credentials: 'same-origin',
    ...init,
  });
