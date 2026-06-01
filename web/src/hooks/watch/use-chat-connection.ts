import { useCallback, useRef, useState } from 'react';
import { parseChatEvent, type ChatMessage } from '../../lib/components/watch/chat-utils.svelte';

const UNREAD_COUNT_ZERO = 0;
const UNREAD_INCREMENT = 1;
const AUTO_SCROLL_THRESHOLD_PX = 32;
const SCROLL_DEBOUNCE_MS = 0;

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
  cleanupConnection: (channelLogin: string) => Promise<void>;
  setChatStatus: (status: string) => void;
  setChatConnected: (connected: boolean) => void;
}

export const useChatConnection = (): UseChatConnectionReturn => {
  const chatMessagesRef = useRef<HTMLDivElement>(null);
  const chatEventsRef = useRef<EventSource | null>(null);

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

  const openChatEvents = useCallback(
    (channelLogin: string): void => {
      if (chatEventsRef.current) {
        chatEventsRef.current.close();
      }

      const eventSource = new EventSource(`/api/chat/events/${encodeURIComponent(channelLogin)}`, {
        withCredentials: true,
      });

      chatEventsRef.current = eventSource;

      eventSource.addEventListener('open', () => {
        setChatConnected(true);
        setChatStatus(`Connected to #${channelLogin}`);
      });

      eventSource.addEventListener('error', () => {
        setChatConnected(false);
        setChatStatus('Chat reconnecting...');
      });

      eventSource.addEventListener('chat', (event: Event) => {
        if (!(event instanceof MessageEvent)) {
          return;
        }
        const messageData: unknown = event.data;
        const messageText = typeof messageData === 'string' ? messageData : String(messageData);
        const message = parseChatEvent(messageText);
        if (message) {
          appendMessage(message);
        }
      });
    },
    [appendMessage],
  );

  const subscribeChat = useCallback(async (channelLogin: string): Promise<void> => {
    const response = await fetch('/api/chat/subscribe', {
      body: JSON.stringify({ channel_login: channelLogin }),
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    });
    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || 'Failed to subscribe to chat');
    }
    setChatStatus(`Connected to #${channelLogin}`);
  }, []);

  const setupConnection = useCallback(
    async (channelLogin: string, chatAvailable: boolean): Promise<void> => {
      if (!chatAvailable) {
        return;
      }
      setChatStatus('Connecting to chat...');
      setChatConnected(false);

      try {
        await subscribeChat(channelLogin);
        openChatEvents(channelLogin);
      } catch {
        setChatStatus('Chat unavailable');
      }
    },
    [subscribeChat, openChatEvents],
  );

  const cleanupConnection = useCallback(async (channelLogin: string): Promise<void> => {
    if (chatEventsRef.current) {
      chatEventsRef.current.close();
      chatEventsRef.current = null;
    }

    if (!channelLogin) {
      return;
    }

    await fetch(`/api/chat/subscribe/${encodeURIComponent(channelLogin)}`, {
      body: JSON.stringify({}),
      credentials: 'same-origin',
      keepalive: true,
      method: 'DELETE',
    });
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
