import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useChat, type ChatStatus } from './use-chat';

const HTTP_NO_CONTENT = 204;
const HTTP_OK = 200;
const EVENT_SOURCE_CONNECTING = 0;
const EVENT_SOURCE_OPEN = 1;
const EVENT_SOURCE_CLOSED = 2;
const EXPECTED_SINGLE_CALL = 1;
const FIRST_EVENT_SOURCE_INDEX = 0;
const EXPECTED_EVENT_URLS = ['/api/chat/events/dj_trico'];
const CONNECTED_STATUS_MESSAGE = 'Connected to #dj_trico';
const UNSUBSCRIBE_GRACE_MS = 3000;

const { getChatEmotesMock } = vi.hoisted(() => ({
  getChatEmotesMock: vi.fn<() => Promise<unknown[]>>(),
}));

vi.mock('../../api-client', () => ({
  getChatEmotes: getChatEmotesMock,
}));

class FakeEventSource extends EventTarget {
  public static readonly CONNECTING = EVENT_SOURCE_CONNECTING;
  public static readonly OPEN = EVENT_SOURCE_OPEN;
  public static readonly CLOSED = EVENT_SOURCE_CLOSED;
  public static openedUrls: string[] = [];
  public static instances: FakeEventSource[] = [];

  public onerror: ((event: Event) => unknown) | null = null;
  public onmessage: ((event: MessageEvent) => unknown) | null = null;
  public onopen: ((event: Event) => unknown) | null = null;
  public readonly readyState = FakeEventSource.OPEN;
  public readonly url: string;
  public readonly withCredentials: boolean;
  public readonly CONNECTING = FakeEventSource.CONNECTING;
  public readonly OPEN = FakeEventSource.OPEN;
  public readonly CLOSED = FakeEventSource.CLOSED;

  public constructor(url: string | URL, init?: EventSourceInit) {
    super();
    this.url = typeof url === 'string' ? url : url.href;
    this.withCredentials = init?.withCredentials === true;
    FakeEventSource.openedUrls.push(this.url);
    FakeEventSource.instances.push(this);
  }

  public close(): void {
    this.dispatchEvent(new Event('close'));
  }
}

const fetchInputUrl = (input: string | URL | Request): string => {
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

describe('useChat', () => {
  let container: HTMLDivElement | null = null;
  let root: Root | null = null;
  let originalEventSource: typeof EventSource | null = null;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
    originalEventSource = globalThis.EventSource;
    vi.stubGlobal('EventSource', FakeEventSource);
    FakeEventSource.openedUrls = [];
    FakeEventSource.instances = [];
    getChatEmotesMock.mockReset();
    getChatEmotesMock.mockResolvedValue([]);
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      await Promise.resolve();
      const url = fetchInputUrl(input);
      if (url === '/api/chat/subscribe' && init?.method === 'POST') {
        return new Response(null, { status: HTTP_NO_CONTENT });
      }
      if (url.startsWith('/api/chat/status')) {
        return Response.json(
          { status: { connected: true, joined: true, subscribed: true } },
          { status: HTTP_OK },
        );
      }
      if (url.startsWith('/api/chat/subscribe/') && init?.method === 'DELETE') {
        return new Response(null, { status: HTTP_NO_CONTENT });
      }
      return new Response(null, { status: HTTP_OK });
    });
  });

  afterEach(() => {
    root?.unmount();
    container?.remove();
    if (originalEventSource) {
      vi.stubGlobal('EventSource', originalEventSource);
    }
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('updates status from waiting to connected after receiving chat', async () => {
    const onStatusChange = vi.fn<(status: ChatStatus) => void>();

    const TestComponent = (): null => {
      useChat({
        channelLogin: 'dj_trico',
        chatAvailable: true,
        onStatusChange,
      });
      return null;
    };

    act(() => {
      root?.render(<TestComponent />);
    });
    await flushAsyncWork();

    act(() => {
      FakeEventSource.instances[FIRST_EVENT_SOURCE_INDEX]?.dispatchEvent(
        new MessageEvent('chat', {
          data: JSON.stringify({
            kind: 'message',
            parts: [{ kind: 'text', text: 'hello' }],
            sender_display_name: 'seraakai',
            sender_login: 'seraakai',
            text: 'hello',
          }),
        }),
      );
    });
    await flushAsyncWork();

    expect(onStatusChange).toHaveBeenLastCalledWith({
      available: true,
      connected: true,
      message: CONNECTED_STATUS_MESSAGE,
    });
  });

  it('does not resubscribe when emote loading updates hook state', async () => {
    const onStatusChange = vi.fn<(status: ChatStatus) => void>();

    const TestComponent = (): null => {
      useChat({
        channelLogin: 'dj_trico',
        chatAvailable: true,
        onStatusChange,
      });
      return null;
    };

    act(() => {
      root?.render(<TestComponent />);
    });
    await flushAsyncWork();

    const subscribeRequests = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) =>
          fetchInputUrl(input) === '/api/chat/subscribe' && init?.method === 'POST',
      );

    expect(subscribeRequests).toHaveLength(EXPECTED_SINGLE_CALL);
    expect(FakeEventSource.openedUrls).toEqual(EXPECTED_EVENT_URLS);
    expect(getChatEmotesMock).toHaveBeenCalledTimes(EXPECTED_SINGLE_CALL);
  });

  it('uses one subscription when retrying before cleanup', async () => {
    vi.useFakeTimers();
    let retryConnection: (() => void) | null = null;
    const onStatusChange = vi.fn<(status: ChatStatus) => void>();

    const TestComponent = ({ chatAvailable }: { chatAvailable: boolean }): null => {
      const { retryConnection: retry } = useChat({
        channelLogin: 'dj_trico',
        chatAvailable,
        onStatusChange,
      });
      retryConnection = retry;
      return null;
    };

    act(() => {
      root?.render(<TestComponent chatAvailable />);
    });
    await flushAsyncWork();
    act(() => {
      retryConnection?.();
    });
    await flushAsyncWork();
    act(() => {
      root?.render(<TestComponent chatAvailable={false} />);
    });
    act(() => {
      vi.advanceTimersByTime(UNSUBSCRIBE_GRACE_MS);
    });
    await flushAsyncWork();

    const subscribeRequests = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) =>
          fetchInputUrl(input) === '/api/chat/subscribe' && init?.method === 'POST',
      );
    const unsubscribeRequests = vi
      .mocked(globalThis.fetch)
      .mock.calls.filter(
        ([input, init]) =>
          fetchInputUrl(input) === '/api/chat/subscribe/dj_trico' && init?.method === 'DELETE',
      );

    expect(subscribeRequests).toHaveLength(EXPECTED_SINGLE_CALL);
    expect(unsubscribeRequests).toHaveLength(EXPECTED_SINGLE_CALL);
  });
});
