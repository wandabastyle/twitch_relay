import { act, useEffect } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useChatConnection } from './use-chat-connection';

const CHANNEL_A = 'channel_a';
const CHANNEL_B = 'channel_b';
const HTTP_NO_CONTENT = 204;
const ZERO_REQUESTS = 0;
const ONE_REQUEST = 1;
const UNSUBSCRIBE_GRACE_MS = 3000;

class FakeEventSource extends EventTarget {
  public constructor() {
    super();
  }

  public close(): void {
    this.dispatchEvent(new Event('close'));
  }
}

const fetchUrl = (input: string | URL | Request): string => {
  if (typeof input === 'string') {
    return input;
  }
  if (input instanceof URL) {
    return input.href;
  }
  return input.url;
};

const flushAsyncWork = async (): Promise<void> => {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  });
};

describe('useChatConnection pending subscription cleanup', () => {
  let container: HTMLDivElement | null = null;
  let root: Root | null = null;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
    vi.useFakeTimers();
    vi.stubGlobal('EventSource', FakeEventSource);
  });

  afterEach(() => {
    root?.unmount();
    container?.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('waits for a delayed subscribe before deleting it after an A to B to A revisit', async () => {
    const subscribeAResponse = Promise.withResolvers<Response>();
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      await Promise.resolve();
      const url = fetchUrl(input);
      if (url === '/api/chat/subscribe' && init?.method === 'POST') {
        if (init.body === JSON.stringify({ channel_login: CHANNEL_A })) {
          const response = await subscribeAResponse.promise;
          return response;
        }
        return new Response(null, { status: HTTP_NO_CONTENT });
      }
      if (init?.method === 'DELETE') {
        return new Response(null, { status: HTTP_NO_CONTENT });
      }
      return Response.json({ status: { joined: true } });
    });

    const TestComponent = ({ channelLogin }: { channelLogin: string }): null => {
      const { cleanupConnection, setupConnection } = useChatConnection();
      useEffect(() => {
        void setupConnection(channelLogin, true);
        return (): void => {
          cleanupConnection(channelLogin);
        };
      }, [channelLogin, cleanupConnection, setupConnection]);
      return null;
    };

    act(() => {
      root?.render(<TestComponent channelLogin={CHANNEL_A} />);
    });
    await flushAsyncWork();
    act(() => {
      root?.render(<TestComponent channelLogin={CHANNEL_B} />);
    });
    await flushAsyncWork();
    act(() => {
      root?.render(<TestComponent channelLogin={CHANNEL_A} />);
    });
    await flushAsyncWork();
    act(() => {
      root?.unmount();
      vi.advanceTimersByTime(UNSUBSCRIBE_GRACE_MS);
    });
    await flushAsyncWork();

    const subscribeA = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) =>
          fetchUrl(input) === '/api/chat/subscribe' &&
          init?.body === JSON.stringify({ channel_login: CHANNEL_A }),
      );
    const unsubscribeA = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) =>
          fetchUrl(input) === `/api/chat/subscribe/${CHANNEL_A}` && init?.method === 'DELETE',
      );
    expect(subscribeA).toHaveLength(ONE_REQUEST);
    expect(unsubscribeA).toHaveLength(ZERO_REQUESTS);

    act(() => {
      subscribeAResponse.resolve(new Response(null, { status: HTTP_NO_CONTENT }));
    });
    await flushAsyncWork();

    const finalUnsubscribeA = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) =>
          fetchUrl(input) === `/api/chat/subscribe/${CHANNEL_A}` && init?.method === 'DELETE',
      );
    expect(finalUnsubscribeA).toHaveLength(ONE_REQUEST);
  });
});
