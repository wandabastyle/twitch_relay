import { useCallback, useRef, useState } from 'react';
import { isObject } from '../../api-client/core';
import { chatErrorMessage, fetchChat } from './chat-request';
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
  const response = await fetchChat(
    statusUrl,
    {
      credentials: 'same-origin',
      signal,
    },
    'Unable to check chat status',
  );
  return parseChatStatusResponse(await response.json());
};

const subscribeChat = async (channelLogin: string): Promise<void> => {
  logChatLifecycle('Subscribing to chat channel', { channelLogin });
  await fetchChat(
    '/api/chat/subscribe',
    {
      body: JSON.stringify({ channel_login: channelLogin }),
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    },
    'Unable to subscribe to chat',
  );
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

interface ChatSubscriptionRefs {
  pending: React.RefObject<Map<string, Promise<void>>>;
  subscribed: React.RefObject<Set<string>>;
  unsubscribeRequested: React.RefObject<Set<string>>;
  unsubscribeTimers: React.RefObject<Map<string, ReturnType<typeof setTimeout>>>;
}

const applyChatChannelStatus = (
  status: ChatChannelStatus,
  channelLogin: string,
  setChatConnected: (connected: boolean) => void,
  setChatStatus: (status: string) => void,
): void => {
  if (status.error !== undefined && status.error !== '') {
    setChatConnected(false);
    setChatStatus(`Chat unavailable: ${status.error}`);
  } else if (status.joined === true) {
    setChatConnected(true);
    setChatStatus(`Connected to #${channelLogin}`);
  } else {
    setChatConnected(false);
    if (status.connected === true && status.subscribed === true) {
      setChatStatus(`IRC connected; joining #${channelLogin}...`);
    } else if (status.subscribed === true) {
      setChatStatus(`Connecting to #${channelLogin}...`);
    } else {
      setChatStatus(`Chat reconnecting for #${channelLogin}...`);
    }
  }
};

const parseChatStatusEvent = (data: unknown): ChatChannelStatus | null => {
  if (typeof data !== 'string') {
    return null;
  }
  try {
    const payload: unknown = JSON.parse(data);
    return isChatChannelStatus(payload) ? payload : parseChatStatusResponse(payload);
  } catch {
    return null;
  }
};

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
    setChatStatus(`Chat transport connected; waiting for IRC #${channelLogin}...`);
  });

  eventSource.addEventListener('error', () => {
    if (!isCurrentEventSource()) {
      return;
    }
    logChatLifecycle('Chat EventSource reconnecting', { channelLogin, generation });
    setChatConnected(false);
    setChatStatus('Chat SSE reconnecting...');
  });

  const handleStatusEvent = (event: Event): void => {
    if (!isCurrentEventSource() || !(event instanceof MessageEvent)) {
      return;
    }
    const status = parseChatStatusEvent(event.data);
    if (status) {
      applyChatChannelStatus(status, channelLogin, setChatConnected, setChatStatus);
    }
  };

  eventSource.addEventListener('status', handleStatusEvent);
  eventSource.addEventListener('connection', handleStatusEvent);
  eventSource.addEventListener('chat', (event: Event) => {
    if (!isCurrentEventSource() || !(event instanceof MessageEvent)) {
      return;
    }
    const messageData: unknown = event.data;
    const messageText = typeof messageData === 'string' ? messageData : String(messageData);
    const message = parseChatEvent(messageText);
    if (message) {
      appendMessage(message);
      // A chat payload proves IRC has joined even on older backends.
      setChatConnected(true);
      setChatStatus(`Connected to #${channelLogin}`);
    }
  });
};

const unsubscribeChat = (channelLogin: string, subscriptions: ChatSubscriptionRefs): void => {
  logChatLifecycle('Unsubscribing from chat channel after grace period', { channelLogin });
  void fetchChat(
    `/api/chat/subscribe/${encodeURIComponent(channelLogin)}`,
    {
      body: JSON.stringify({}),
      credentials: 'same-origin',
      keepalive: true,
      method: 'DELETE',
    },
    'Unable to unsubscribe from chat',
  )
    .catch((error: unknown) => {
      logChatLifecycle('Chat unsubscribe failed', { channelLogin, error });
    })
    .finally(() => {
      subscriptions.subscribed.current.delete(channelLogin);
      subscriptions.unsubscribeRequested.current.delete(channelLogin);
    });
};

const scheduleChatUnsubscribe = (
  channelLogin: string,
  subscriptions: ChatSubscriptionRefs,
): void => {
  const existingTimer = subscriptions.unsubscribeTimers.current.get(channelLogin);
  if (existingTimer) {
    clearTimeout(existingTimer);
  }

  const timer = setTimeout(() => {
    subscriptions.unsubscribeTimers.current.delete(channelLogin);
    const pendingSubscription = subscriptions.pending.current.get(channelLogin);
    if (pendingSubscription) {
      const finishUnsubscribe = (): void => {
        if (subscriptions.unsubscribeRequested.current.has(channelLogin)) {
          unsubscribeChat(channelLogin, subscriptions);
        }
      };
      void pendingSubscription.then(finishUnsubscribe, finishUnsubscribe);
      return;
    }
    if (!subscriptions.unsubscribeRequested.current.has(channelLogin)) {
      return;
    }
    unsubscribeChat(channelLogin, subscriptions);
  }, UNSUBSCRIBE_GRACE_MS);

  subscriptions.unsubscribeTimers.current.set(channelLogin, timer);
};

const cancelChatUnsubscribe = (channelLogin: string, subscriptions: ChatSubscriptionRefs): void => {
  const pendingTimer = subscriptions.unsubscribeTimers.current.get(channelLogin);
  if (pendingTimer) {
    clearTimeout(pendingTimer);
    subscriptions.unsubscribeTimers.current.delete(channelLogin);
    logChatLifecycle('Canceled pending chat unsubscribe', { channelLogin });
  }
  subscriptions.unsubscribeRequested.current.delete(channelLogin);
};

const ensureChatSubscription = async (
  channelLogin: string,
  subscriptions: ChatSubscriptionRefs,
): Promise<void> => {
  const pendingSubscription = subscriptions.pending.current.get(channelLogin);
  if (pendingSubscription) {
    await pendingSubscription;
    return;
  }
  if (subscriptions.subscribed.current.has(channelLogin)) {
    return;
  }

  subscriptions.subscribed.current.add(channelLogin);
  const subscription = subscribeChat(channelLogin);
  subscriptions.pending.current.set(channelLogin, subscription);
  void subscription
    .catch(() => {
      if (!subscriptions.unsubscribeRequested.current.has(channelLogin)) {
        subscriptions.subscribed.current.delete(channelLogin);
      }
    })
    .finally(() => {
      subscriptions.pending.current.delete(channelLogin);
    });
  await subscription;
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
  resetChannelState: () => void;
  setChatStatus: (status: string) => void;
  setChatConnected: (connected: boolean) => void;
}

export const useChatConnection = (): UseChatConnectionReturn => {
  const chatMessagesRef = useRef<HTMLDivElement>(null);
  const chatEventsRef = useRef<EventSource | null>(null);
  const connectionGenerationRef = useRef(INITIAL_CONNECTION_GENERATION);
  const subscribeAbortRef = useRef<AbortController | null>(null);
  const pendingSubscriptionsRef = useRef<Map<string, Promise<void>>>(new Map());
  const subscribedChannelsRef = useRef<Set<string>>(new Set());
  const unsubscribeRequestedChannelsRef = useRef<Set<string>>(new Set());
  const unsubscribeTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const subscriptions: ChatSubscriptionRefs = {
    pending: pendingSubscriptionsRef,
    subscribed: subscribedChannelsRef,
    unsubscribeRequested: unsubscribeRequestedChannelsRef,
    unsubscribeTimers: unsubscribeTimersRef,
  };

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
      cancelChatUnsubscribe(channelLogin, subscriptions);

      const generation = connectionGenerationRef.current + NEXT_CONNECTION_GENERATION_INCREMENT;
      connectionGenerationRef.current = generation;
      subscribeAbortRef.current?.abort();
      const abortController = new AbortController();
      subscribeAbortRef.current = abortController;

      setChatStatus(`Subscribing to #${channelLogin}...`);
      setChatConnected(false);

      try {
        await ensureChatSubscription(channelLogin, subscriptions);
        if (connectionGenerationRef.current !== generation) {
          return;
        }
        const status = await readChatStatus(channelLogin, abortController.signal);
        if (connectionGenerationRef.current !== generation) {
          return;
        }
        if (status) {
          applyChatChannelStatus(status, channelLogin, setChatConnected, setChatStatus);
        } else {
          setChatStatus(`Opening chat transport for #${channelLogin}...`);
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
        setChatStatus(
          `Chat unavailable: ${chatErrorMessage(error, 'Unable to subscribe to chat')}`,
        );
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

    if (!channelLogin || !subscriptions.subscribed.current.has(channelLogin)) {
      return;
    }

    subscriptions.unsubscribeRequested.current.add(channelLogin);
    scheduleChatUnsubscribe(channelLogin, subscriptions);
  }, []);

  const resetChannelState = useCallback((): void => {
    setChatMessages([]);
    setUnreadChatCount(UNREAD_COUNT_ZERO);
    setChatConnected(false);
    setChatStatus('Checking Twitch chat...');
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
    resetChannelState,
    setChatConnected,
    setChatStatus,
    setupConnection,
    unreadChatCount,
  };
};
