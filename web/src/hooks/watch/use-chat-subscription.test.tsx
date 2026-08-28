import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useChat, type ChatStatus } from './use-chat';

const CHANNEL_A = 'channel_a';
const CHANNEL_B = 'channel_b';
const HTTP_NO_CONTENT = 204;
const ONE_REQUEST = 1;
const TWO_REQUESTS = 2;
const UNSUBSCRIBE_GRACE_MS = 3000;

vi.mock('../../api-client', () => ({ getChatEmotes: vi.fn().mockResolvedValue([]) }));

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

describe('useChat subscription ownership', () => {
  let container: HTMLDivElement | null = null;
  let root: Root | null = null;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
    vi.useFakeTimers();
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      await Promise.resolve();
      if (fetchUrl(input) === '/api/chat/subscribe' && init?.method === 'POST') {
        return new Response(null, { status: HTTP_NO_CONTENT });
      }
      if (init?.method === 'DELETE') {
        return new Response(null, { status: HTTP_NO_CONTENT });
      }
      return Response.json({ status: { connected: true, joined: true, subscribed: true } });
    });
  });

  afterEach(() => {
    root?.unmount();
    container?.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('keeps channel ownership across an A to B to A revisit', async () => {
    const onStatusChange = vi.fn<(status: ChatStatus) => void>();
    const TestComponent = ({ channelLogin }: { channelLogin: string }): null => {
      useChat({ channelLogin, chatAvailable: true, onStatusChange });
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
    });
    act(() => {
      vi.advanceTimersByTime(UNSUBSCRIBE_GRACE_MS);
    });
    await flushAsyncWork();

    const subscribeA = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) => fetchUrl(input) === '/api/chat/subscribe' && init?.method === 'POST',
      )
      .filter(([, init]) => init?.body === JSON.stringify({ channel_login: CHANNEL_A }));
    const subscribeB = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) => fetchUrl(input) === '/api/chat/subscribe' && init?.method === 'POST',
      )
      .filter(([, init]) => init?.body === JSON.stringify({ channel_login: CHANNEL_B }));
    const unsubscribeA = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) =>
          fetchUrl(input) === `/api/chat/subscribe/${CHANNEL_A}` && init?.method === 'DELETE',
      );
    const unsubscribeB = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) =>
          fetchUrl(input) === `/api/chat/subscribe/${CHANNEL_B}` && init?.method === 'DELETE',
      );

    expect(subscribeA).toHaveLength(ONE_REQUEST);
    expect(subscribeB).toHaveLength(ONE_REQUEST);
    expect(unsubscribeA).toHaveLength(ONE_REQUEST);
    expect(unsubscribeB).toHaveLength(ONE_REQUEST);
    expect(subscribeA.length + subscribeB.length).toBe(TWO_REQUESTS);
    expect(unsubscribeA.length + unsubscribeB.length).toBe(TWO_REQUESTS);
  });
});
