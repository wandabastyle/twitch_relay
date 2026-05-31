import { useCallback, useEffect, useRef, useState } from 'react';
import { getChatEmotes, type EmoteItem } from '../../api-client';
import { parseChatEvent, type ChatMessage } from '../../lib/components/watch/chat-utils.svelte';

export interface ChatStatus {
  available: boolean;
  connected: boolean;
  message: string;
}

export interface UseChatReturn {
  chatMessages: ChatMessage[];
  chatConnected: boolean;
  chatStatus: string;
  chatSending: boolean;
  unreadChatCount: number;
  localEmotes: EmoteItem[];
  emotesLoaded: boolean;
  chatMessagesRef: React.RefObject<HTMLDivElement | null>;
  sendMessage: (text: string) => Promise<void>;
  handleScroll: () => void;
  jumpToLatest: () => void;
  insertEmote: (code: string) => void;
  onComposerSelect: (code: string) => void;
}

export interface UseChatOptions {
  channelLogin: string;
  chatAvailable: boolean;
  initialEmotes?: EmoteItem[];
  onStatusChange: (status: ChatStatus) => void;
}

const AUTO_SCROLL_THRESHOLD_PX = 32;
const SCROLL_DEBOUNCE_MS = 0;
const UNREAD_COUNT_ZERO = 0;

export const useChat = (options: UseChatOptions): UseChatReturn => {
  const { channelLogin, chatAvailable, initialEmotes = [], onStatusChange } = options;

  const chatMessagesRef = useRef<HTMLDivElement>(null);
  const chatEventsRef = useRef<EventSource | null>(null);
  const composerRef = useRef<{ insertEmote?: (code: string) => void }>(null);

  const [localEmotes, setLocalEmotes] = useState<EmoteItem[]>([]);
  const [emotesLoaded, setEmotesLoaded] = useState(false);
  const [chatConnected, setChatConnected] = useState(false);
  const [chatStatus, setChatStatus] = useState('Checking Twitch chat...');
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [unreadChatCount, setUnreadChatCount] = useState(UNREAD_COUNT_ZERO);
  const [chatSending, setChatSending] = useState(false);

  // Initialize emotes from props
  useEffect(() => {
    if (initialEmotes.length > UNREAD_COUNT_ZERO && !emotesLoaded) {
      setLocalEmotes(initialEmotes);
      setEmotesLoaded(true);
    }
  }, [initialEmotes, emotesLoaded]);

  // Notify status changes
  useEffect(() => {
    onStatusChange({ available: chatAvailable, connected: chatConnected, message: chatStatus });
  }, [chatAvailable, chatConnected, chatStatus, onStatusChange]);

  // Scroll helpers
  const isNearBottom = useCallback((): boolean => {
    const el = chatMessagesRef.current;
    if (!el) {
      return true;
    }
    const distance = el.scrollHeight - el.clientHeight - el.scrollTop;
    return distance <= AUTO_SCROLL_THRESHOLD_PX;
  }, []);

  // Chat operations
  const appendMessage = useCallback(
    (message: ChatMessage): void => {
      const UNREAD_INCREMENT = 1;
      const shouldStickToBottom = isNearBottom();
      setChatMessages((prev) => [...prev, message]);

      // Use setTimeout to wait for DOM update
      setTimeout(() => {
        if (shouldStickToBottom && chatMessagesRef.current) {
          chatMessagesRef.current.scrollTop = chatMessagesRef.current.scrollHeight;
          setUnreadChatCount(UNREAD_COUNT_ZERO);
        } else {
          setUnreadChatCount((prev) => prev + UNREAD_INCREMENT);
        }
      }, SCROLL_DEBOUNCE_MS);
    },
    [isNearBottom],
  );

  const loadEmotes = useCallback(async (): Promise<void> => {
    if (emotesLoaded || !channelLogin) {
      return;
    }

    const emotes = await getChatEmotes(channelLogin);
    setLocalEmotes(emotes);
    setEmotesLoaded(true);
  }, [channelLogin, emotesLoaded]);

  const sendMessage = useCallback(
    async (text: string): Promise<void> => {
      const trimmed = text.trim();
      if (trimmed.length === UNREAD_COUNT_ZERO) {
        return;
      }

      setChatSending(true);

      try {
        const response = await fetch('/api/chat/send', {
          body: JSON.stringify({ channel_login: channelLogin, message: trimmed }),
          credentials: 'same-origin',
          headers: { 'content-type': 'application/json' },
          method: 'POST',
        });

        if (!response.ok) {
          const error = await response.text();
          throw new Error(error || 'Failed to send message');
        }
        setChatStatus(`Connected to #${channelLogin}`);
      } catch (error: unknown) {
        const message = error instanceof Error ? error.message : 'Failed to send message';
        setChatStatus(message);
        throw error;
      } finally {
        setChatSending(false);
      }
    },
    [channelLogin],
  );

  const subscribeChat = useCallback(async (): Promise<void> => {
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
  }, [channelLogin]);

  const cleanupChat = useCallback(async (): Promise<void> => {
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
  }, [channelLogin]);

  // Event handlers
  const handleChatEvent = useCallback(
    (message: ChatMessage): void => {
      appendMessage(message);
    },
    [appendMessage],
  );

  const openChatEvents = useCallback((): void => {
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
      const message = parseChatEvent(event.data);
      if (message) {
        appendMessage(message);
      }
    });
  }, [channelLogin, handleChatEvent]);

  const setupChat = useCallback(async (): Promise<void> => {
    setChatStatus('Connecting to chat...');
    setChatConnected(false);

    try {
      await subscribeChat();
      openChatEvents();
      try {
        await loadEmotes();
      } catch {
        // Ignore emote loading errors
      }
    } catch {
      setChatStatus('Chat unavailable');
    }
  }, [subscribeChat, openChatEvents, loadEmotes]);

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

  const onComposerSelect = useCallback((code: string): void => {
    composerRef.current?.insertEmote?.(code);
  }, []);

  const insertEmote = useCallback((code: string): void => {
    // This is handled by the composer component
    composerRef.current?.insertEmote?.(code);
  }, []);

  // Setup chat on mount
  useEffect(() => {
    if (chatAvailable) {
      void setupChat();
    }

    return (): void => {
      void cleanupChat();
    };
  }, [chatAvailable, setupChat, cleanupChat]);

  return {
    chatConnected,
    chatMessages,
    chatMessagesRef,
    chatSending,
    chatStatus,
    emotesLoaded,
    handleScroll,
    insertEmote,
    jumpToLatest,
    localEmotes,
    onComposerSelect,
    sendMessage,
    unreadChatCount,
  };
}
