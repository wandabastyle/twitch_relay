import { useCallback, useRef, useState } from 'react';
import { isObject } from '../../api-client/core';
import { parseChatEvent, type ChatMessage } from './chat-utils';

const UNREAD_COUNT_ZERO = 0;
const UNREAD_INCREMENT = 1;
const AUTO_SCROLL_THRESHOLD_PX = 32;
const SCROLL_DEBOUNCE_MS = 0;
const UNSUBSCRIBE_GRACE_MS = 3000;
const INITIAL_CONNECTION_GENERATION = 0;
const NEXT_CONNECTION_GENERATION_INCREMENT = 1;

const logChatLifecycle = (message: string, detail: Record<string, unknown>): void => {
  globalThis.dispatchEvent(new CustomEvent('chat:lifecycle', { detail: { message, ...detail } }));
};

interface ChatChannelStatus {
  connected?: boolean;
  error?: string;
  joined?: boolean;
  subscribed?: boolean;
}

const isChatChannelStatus = (value: unknown): value is ChatChannelStatus => {
  if (!isObject(value)) {
    return false;
  }
  const candidate = value;
  return (
    (candidate.connected === undefined || typeof candidate.connected === 'boolean') &&
    (candidate.error === undefined || typeof candidate.error === 'string') &&
    (candidate.joined === undefined || typeof candidate.joined === 'boolean') &&
    (candidate.subscribed === undefined || typeof candidate.subscribed === 'boolean')
  );
};

const parseChatStatusResponse = (payload: unknown): ChatChannelStatus | null => {
  if (!isObject(payload)) {
    return null;
  }
  const { status } = payload;
  return isChatChannelStatus(status) ? status : null;
};

const readChatStatus = async (
  channelLogin: string,
  signal: AbortSignal,
): Promise<ChatChannelStatus | null> => {
  const statusUrl = `/api/chat/status?channel_login=${encodeURIComponent(channelLogin)}`;
  const response = await fetch(statusUrl, {
    credentials: 'same-origin',
    signal,
  });
  if (!response.ok) {
    return null;
  }
  return parseChatStatusResponse(await response.json());
};

const subscribeChat = async (channelLogin: string, signal: AbortSignal): Promise<void> => {
  logChatLifecycle('Subscribing to chat channel', { channelLogin });
  const response = await fetch('/api/chat/subscribe', {
    body: JSON.stringify({ channel_login: channelLogin }),
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json' },
    method: 'POST',
    signal,
  });
  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || 'Failed to subscribe to chat');
  }
  logChatLifecycle('Chat subscribe accepted', { channelLogin });
};

interface OpenChatEventsOptions {
  appendMessage: (message: ChatMessage) => void;
  channelLogin: string;
  chatEventsRef: React.RefObject<EventSource | null>;
  connectionGenerationRef: React.RefObject<number>;
  generation: number;
  setChatConnected: (connected: boolean) => void;
  setChatStatus: (status: string) => void;
}

const openChatEventsForConnection = (options: OpenChatEventsOptions): void => {
  const {
    appendMessage,
    channelLogin,
    chatEventsRef,
    connectionGenerationRef,
    generation,
    setChatConnected,
    setChatStatus,
  } = options;

  if (chatEventsRef.current) {
    chatEventsRef.current.close();
  }

  const eventsUrl = `/api/chat/events/${encodeURIComponent(channelLogin)}`;
  logChatLifecycle('Opening chat EventSource', { channelLogin, generation });
  const eventSource = new EventSource(eventsUrl, {
    withCredentials: true,
  });

  chatEventsRef.current = eventSource;

  const isCurrentEventSource = (): boolean =>
    chatEventsRef.current === eventSource && connectionGenerationRef.current === generation;

  eventSource.addEventListener('open', () => {
    if (!isCurrentEventSource()) {
      return;
    }
    logChatLifecycle('Chat EventSource opened', { channelLogin, generation });
    setChatConnected(true);
    setChatStatus(`SSE connected to #${channelLogin}; waiting for IRC messages`);
  });

  eventSource.addEventListener('error', () => {
    if (!isCurrentEventSource()) {
      return;
    }
    logChatLifecycle('Chat EventSource reconnecting', { channelLogin, generation });
    setChatConnected(false);
    setChatStatus('Chat SSE reconnecting...');
  });

  eventSource.addEventListener('chat', (event: Event) => {
    if (!isCurrentEventSource() || !(event instanceof MessageEvent)) {
      return;
    }
    const messageData: unknown = event.data;
    const messageText = typeof messageData === 'string' ? messageData : String(messageData);
    const message = parseChatEvent(messageText);
    if (message) {
      appendMessage(message);
    }
  });
};

const scheduleChatUnsubscribe = (
  channelLogin: string,
  unsubscribeTimersRef: React.RefObject<Map<string, ReturnType<typeof setTimeout>>>,
): void => {
  const existingTimer = unsubscribeTimersRef.current.get(channelLogin);
  if (existingTimer) {
    clearTimeout(existingTimer);
  }

  const timer = setTimeout(() => {
    unsubscribeTimersRef.current.delete(channelLogin);
    logChatLifecycle('Unsubscribing from chat channel after grace period', { channelLogin });
    void fetch(`/api/chat/subscribe/${encodeURIComponent(channelLogin)}`, {
      body: JSON.stringify({}),
      credentials: 'same-origin',
      keepalive: true,
      method: 'DELETE',
    });
  }, UNSUBSCRIBE_GRACE_MS);

  unsubscribeTimersRef.current.set(channelLogin, timer);
};

export interface UseChatConnectionReturn {
  chatConnected: boolean;
  chatStatus: string;
  chatMessages: ChatMessage[];
  unreadChatCount: number;
  chatMessagesRef: React.RefObject<HTMLDivElement | null>;
  chatEventsRef: React.RefObject<EventSource | null>;
  handleScroll: () => void;
  jumpToLatest: () => void;
  clearUnreadCount: () => void;
  appendMessage: (message: ChatMessage) => void;
  setupConnection: (channelLogin: string, chatAvailable: boolean) => Promise<void>;
  cleanupConnection: (channelLogin: string) => void;
  setChatStatus: (status: string) => void;
  setChatConnected: (connected: boolean) => void;
}

export const useChatConnection = (): UseChatConnectionReturn => {
  const chatMessagesRef = useRef<HTMLDivElement>(null);
  const chatEventsRef = useRef<EventSource | null>(null);
  const connectionGenerationRef = useRef(INITIAL_CONNECTION_GENERATION);
  const subscribeAbortRef = useRef<AbortController | null>(null);
  const unsubscribeTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const [chatConnected, setChatConnected] = useState(false);
  const [chatStatus, setChatStatus] = useState('Checking Twitch chat...');
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [unreadChatCount, setUnreadChatCount] = useState(UNREAD_COUNT_ZERO);

  const isNearBottom = useCallback((): boolean => {
    const el = chatMessagesRef.current;
    if (!el) {
      return true;
    }
    const distance = el.scrollHeight - el.clientHeight - el.scrollTop;
    return distance <= AUTO_SCROLL_THRESHOLD_PX;
  }, []);

  const clearUnreadCount = useCallback((): void => {
    setUnreadChatCount(UNREAD_COUNT_ZERO);
  }, []);

  const appendMessage = useCallback(
    (message: ChatMessage): void => {
      const shouldStickToBottom = isNearBottom();
      setChatMessages((prev) => [...prev, message]);

      setTimeout(() => {
        if (chatMessagesRef.current) {
          if (shouldStickToBottom) {
            chatMessagesRef.current.scrollTop = chatMessagesRef.current.scrollHeight;
            setUnreadChatCount(UNREAD_COUNT_ZERO);
          } else {
            setUnreadChatCount((prev) => prev + UNREAD_INCREMENT);
          }
        }
      }, SCROLL_DEBOUNCE_MS);
    },
    [isNearBottom],
  );

  const handleScroll = useCallback((): void => {
    if (isNearBottom()) {
      setUnreadChatCount(UNREAD_COUNT_ZERO);
    }
  }, [isNearBottom]);

  const jumpToLatest = useCallback((): void => {
    if (chatMessagesRef.current) {
      chatMessagesRef.current.scrollTop = chatMessagesRef.current.scrollHeight;
      setUnreadChatCount(UNREAD_COUNT_ZERO);
    }
  }, []);

  const setupConnection = useCallback(
    async (channelLogin: string, chatAvailable: boolean): Promise<void> => {
      if (!chatAvailable) {
        return;
      }
      const pendingTimer = unsubscribeTimersRef.current.get(channelLogin);
      if (pendingTimer) {
        clearTimeout(pendingTimer);
        unsubscribeTimersRef.current.delete(channelLogin);
        logChatLifecycle('Canceled pending chat unsubscribe', { channelLogin });
      }

      const generation = connectionGenerationRef.current + NEXT_CONNECTION_GENERATION_INCREMENT;
      connectionGenerationRef.current = generation;
      subscribeAbortRef.current?.abort();
      const abortController = new AbortController();
      subscribeAbortRef.current = abortController;

      setChatStatus(`Subscribing to #${channelLogin}...`);
      setChatConnected(false);

      try {
        await subscribeChat(channelLogin, abortController.signal);
        if (connectionGenerationRef.current !== generation) {
          return;
        }
        const status = await readChatStatus(channelLogin, abortController.signal);
        if (connectionGenerationRef.current !== generation) {
          return;
        }
        if (status?.joined === true) {
          setChatStatus(`IRC joined #${channelLogin}; opening chat SSE...`);
        } else if (status?.connected === true && status.subscribed === true) {
          setChatStatus(`IRC connected; joining #${channelLogin}...`);
        } else if (typeof status?.error === 'string' && status.error.length > UNREAD_COUNT_ZERO) {
          setChatStatus(`Chat unavailable: ${status.error}`);
        } else {
          setChatStatus(`Opening chat SSE for #${channelLogin}...`);
        }
        openChatEventsForConnection({
          appendMessage,
          channelLogin,
          chatEventsRef,
          connectionGenerationRef,
          generation,
          setChatConnected,
          setChatStatus,
        });
      } catch (error: unknown) {
        if (abortController.signal.aborted || connectionGenerationRef.current !== generation) {
          return;
        }
        logChatLifecycle('Chat subscribe failed', { channelLogin, error });
        setChatConnected(false);
        setChatStatus('Chat unavailable');
      }
    },
    [appendMessage],
  );

  const cleanupConnection = useCallback((channelLogin: string): void => {
    connectionGenerationRef.current += NEXT_CONNECTION_GENERATION_INCREMENT;
    subscribeAbortRef.current?.abort();
    subscribeAbortRef.current = null;

    if (chatEventsRef.current) {
      logChatLifecycle('Closing chat EventSource', { channelLogin });
      chatEventsRef.current.close();
      chatEventsRef.current = null;
    }

    setChatConnected(false);

    if (!channelLogin) {
      return;
    }

    scheduleChatUnsubscribe(channelLogin, unsubscribeTimersRef);
  }, []);

  return {
    appendMessage,
    chatConnected,
    chatEventsRef,
    chatMessages,
    chatMessagesRef,
    chatStatus,
    cleanupConnection,
    clearUnreadCount,
    handleScroll,
    jumpToLatest,
    setChatConnected,
    setChatStatus,
    setupConnection,
    unreadChatCount,
  };
};
