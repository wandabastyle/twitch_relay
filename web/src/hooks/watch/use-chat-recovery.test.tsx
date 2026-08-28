import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useChat, type ChatStatus } from './use-chat';

const HTTP_NO_CONTENT = 204;
const HTTP_OK = 200;
const ZERO = 0;
const ONE = 1;

vi.mock('../../api-client', () => ({ getChatEmotes: vi.fn().mockResolvedValue([]) }));

class FakeEventSource extends EventTarget {
  public static instances: FakeEventSource[] = [];

  public constructor() {
    super();
    FakeEventSource.instances.push(this);
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

const flush = async (): Promise<void> => {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  });
};

describe('useChat recovery', () => {
  let container: HTMLDivElement | null = null;
  let root: Root | null = null;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
    FakeEventSource.instances = [];
    vi.stubGlobal('EventSource', FakeEventSource);
  });

  afterEach(() => {
    root?.unmount();
    container?.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('reports a failed subscription without opening an EventSource', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('subscription refused', { status: 400 }),
    );
    const onStatusChange = vi.fn<(status: ChatStatus) => void>();
    const TestComponent = (): null => {
      useChat({ channelLogin: 'dj_trico', chatAvailable: true, onStatusChange });
      return null;
    };

    act(() => {
      root?.render(<TestComponent />);
    });
    await flush();

    expect(FakeEventSource.instances).toHaveLength(ZERO);
    expect(onStatusChange).toHaveBeenLastCalledWith({
      available: true,
      connected: false,
      message: 'Chat unavailable: subscription refused',
    });
  });

  it('keeps chat disconnected when only the SSE transport opens', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      await Promise.resolve();
      const url = fetchUrl(input);
      if (url === '/api/chat/subscribe' && init?.method === 'POST') {
        return new Response(null, { status: HTTP_NO_CONTENT });
      }
      if (url.startsWith('/api/chat/status')) {
        return Response.json({ status: { connected: true, joined: false, subscribed: true } });
      }
      return new Response(null, { status: HTTP_OK });
    });
    const onStatusChange = vi.fn<(status: ChatStatus) => void>();
    const TestComponent = (): null => {
      useChat({ channelLogin: 'dj_trico', chatAvailable: true, onStatusChange });
      return null;
    };

    act(() => {
      root?.render(<TestComponent />);
    });
    await flush();
    act(() => {
      FakeEventSource.instances[ZERO]?.dispatchEvent(new Event('open'));
    });
    await flush();

    expect(onStatusChange).toHaveBeenLastCalledWith({
      available: true,
      connected: false,
      message: 'Chat transport connected; waiting for IRC #dj_trico...',
    });
  });

  it('resets messages when the channel changes', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      await Promise.resolve();
      if (fetchUrl(input) === '/api/chat/subscribe' && init?.method === 'POST') {
        return new Response(null, { status: HTTP_NO_CONTENT });
      }
      return Response.json({ status: { connected: true, joined: true, subscribed: true } });
    });
    const onStatusChange = vi.fn<(status: ChatStatus) => void>();
    let chatMessages: readonly { text: string }[] = [];
    const TestComponent = ({ channelLogin }: { channelLogin: string }): null => {
      const { chatMessages: messages } = useChat({
        channelLogin,
        chatAvailable: true,
        onStatusChange,
      });
      chatMessages = messages;
      return null;
    };

    act(() => {
      root?.render(<TestComponent channelLogin="dj_trico" />);
    });
    await flush();
    act(() => {
      FakeEventSource.instances[ZERO]?.dispatchEvent(
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
    await flush();
    expect(chatMessages).toHaveLength(ONE);

    act(() => {
      root?.render(<TestComponent channelLogin="another_channel" />);
    });
    await flush();

    expect(chatMessages).toHaveLength(ZERO);
  });
});
