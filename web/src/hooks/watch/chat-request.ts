const CHAT_REQUEST_TIMEOUT_MS = 10_000;

const responseError = async (response: Response, fallback: string): Promise<Error> => {
  const responseText = await response.text();
  const detail = responseText.trim();
  return new Error(detail === '' ? `${fallback} (${response.status})` : detail);
};

export const fetchChat = async (
  input: RequestInfo | URL,
  init: RequestInit,
  fallback: string,
): Promise<Response> => {
  const controller = new AbortController();
  const timeout = globalThis.setTimeout(() => {
    controller.abort();
  }, CHAT_REQUEST_TIMEOUT_MS);
  const abortRequest = (): void => {
    controller.abort();
  };
  init.signal?.addEventListener('abort', abortRequest, { once: true });

  try {
    const response = await fetch(input, { ...init, signal: controller.signal });
    if (!response.ok) {
      throw await responseError(response, fallback);
    }
    return response;
  } catch (error: unknown) {
    if (controller.signal.aborted && init.signal?.aborted !== true) {
      throw new Error(`${fallback}: request timed out`, { cause: error });
    }
    if (error instanceof TypeError) {
      throw new TypeError(`${fallback}: network unavailable`, { cause: error });
    }
    throw error;
  } finally {
    globalThis.clearTimeout(timeout);
    init.signal?.removeEventListener('abort', abortRequest);
  }
};

export const chatErrorMessage = (error: unknown, fallback: string): string =>
  error instanceof Error && error.message.trim() !== '' ? error.message : fallback;
