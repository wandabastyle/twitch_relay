import { useCallback, useRef, useState } from 'react';
import { getChatEmotes, type EmoteItem } from '../../api-client';
import { parseChatEvent, type ChatMessage } from '../../lib/components/watch/chat-utils.svelte';

const AUTO_SCROLL_THRESHOLD_PX = 32;
const SCROLL_DEBOUNCE_MS = 0;
const UNREAD_COUNT_ZERO = 0;
const UNREAD_INCREMENT = 1;

export interface ChatMessagesState {
  chatMessages: ChatMessage[];
  unreadChatCount: number;
  chatConnected: boolean;
  chatStatus: string;
  localEmotes: EmoteItem[];
  emotesLoaded: boolean;
}

export interface ChatMessagesActions {
  setChatMessages: (messages: ChatMessage[] | ((prev: ChatMessage[]) => ChatMessage[])) => void;
  setUnreadChatCount: (count: number | ((prev: number) => number)) => void;
  setChatConnected: (connected: boolean) => void;
  setChatStatus: (status: string) => void;
  setLocalEmotes: (emotes: EmoteItem[]) => void;
  setEmotesLoaded: (loaded: boolean) => void;
}

export interface UseChatMessagesOptions {
  channelLogin: string;
  initialEmotes?: EmoteItem[];
}

export interface UseChatMessagesReturn {
  chatMessages: ChatMessage[];
  unreadChatCount: number;
  localEmotes: EmoteItem[];
  emotesLoaded: boolean;
  appendMessage: (message: ChatMessage) => void;
  loadEmotes: () => Promise<void>;
  handleChatEvent: (message: ChatMessage) => void;
  isNearBottom: (element: HTMLDivElement | null) => boolean;
  resetUnreadCount: () => void;
}

export const useChatMessages = (options: UseChatMessagesOptions): UseChatMessagesReturn => {
  const { channelLogin, initialEmotes = [] } = options;

  const [localEmotes, setLocalEmotes] = useState<EmoteItem[]>([]);
  const [emotesLoaded, setEmotesLoaded] = useState(false);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [unreadChatCount, setUnreadChatCount] = useState(UNREAD_COUNT_ZERO);

  // Initialize emotes from props
  const initializedRef = useRef(false);
  if (!initializedRef.current && initialEmotes.length > UNREAD_COUNT_ZERO && !emotesLoaded) {
    initializedRef.current = true;
    setLocalEmotes(initialEmotes);
    setEmotesLoaded(true);
  }

  // Scroll helpers
  const isNearBottom = useCallback((element: HTMLDivElement | null): boolean => {
    if (!element) {
      return true;
    }
    const distance = element.scrollHeight - element.clientHeight - element.scrollTop;
    return distance <= AUTO_SCROLL_THRESHOLD_PX;
  }, []);

  const resetUnreadCount = useCallback((): void => {
    setUnreadChatCount(UNREAD_COUNT_ZERO);
  }, []);

  // Chat operations
  const appendMessage = useCallback(
    (message: ChatMessage, messagesContainerRef?: React.RefObject<HTMLDivElement | null>): void => {
      const shouldStickToBottom = messagesContainerRef?.current
        ? isNearBottom(messagesContainerRef.current)
        : true;

      setChatMessages((prev) => [...prev, message]);

      // Use setTimeout to wait for DOM update
      setTimeout(() => {
        if (messagesContainerRef?.current) {
          if (shouldStickToBottom) {
            messagesContainerRef.current.scrollTop = messagesContainerRef.current.scrollHeight;
            setUnreadChatCount(UNREAD_COUNT_ZERO);
          } else {
            setUnreadChatCount((prev) => prev + UNREAD_INCREMENT);
          }
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

  const handleChatEvent = useCallback(
    (message: ChatMessage): void => {
      appendMessage(message);
    },
    [appendMessage],
  );

  return {
    appendMessage,
    chatMessages,
    emotesLoaded,
    handleChatEvent,
    isNearBottom,
    loadEmotes,
    localEmotes,
    resetUnreadCount,
    unreadChatCount,
  };
};

export interface MessageHandlerOptions {
  setChatMessages: (messages: ChatMessage[] | ((prev: ChatMessage[]) => ChatMessage[])) => void;
  setUnreadChatCount: (count: number | ((prev: number) => number)) => void;
  chatMessagesRef: React.RefObject<HTMLDivElement | null>;
  isNearBottom: () => boolean;
}

export interface CreateMessageHandlerReturn {
  appendMessage: (message: ChatMessage) => void;
  handleChatEvent: (message: ChatMessage) => void;
}

export const createMessageHandler = (
  options: MessageHandlerOptions,
): CreateMessageHandlerReturn => {
  const { setChatMessages, setUnreadChatCount, chatMessagesRef, isNearBottom } = options;

  const appendMessage = (message: ChatMessage): void => {
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
  };

  const handleChatEvent = (message: ChatMessage): void => {
    appendMessage(message);
  };

  return {
    appendMessage,
    handleChatEvent,
  };
};

export interface EventSourceHandlerOptions {
  channelLogin: string;
  onMessage: (message: ChatMessage) => void;
  onConnected: () => void;
  onError: () => void;
  setChatStatus: (status: string) => void;
}

export interface CreateEventSourceHandlersReturn {
  handleChatEvent: (event: Event) => void;
  handleError: () => void;
  handleOpen: () => void;
}

export const createEventSourceHandlers = (
  options: EventSourceHandlerOptions,
): CreateEventSourceHandlersReturn => {
  const { channelLogin, onMessage, onConnected, onError, setChatStatus } = options;

  const handleOpen = (): void => {
    onConnected();
    setChatStatus(`Connected to #${channelLogin}`);
  };

  const handleError = (): void => {
    onError();
    setChatStatus('Chat reconnecting...');
  };

  const handleChatEvent = (event: Event): void => {
    if (!(event instanceof MessageEvent)) {
      return;
    }
    const messageData: unknown = event.data;
    const messageText = typeof messageData === 'string' ? messageData : String(messageData);
    const message = parseChatEvent(messageText);
    if (message) {
      onMessage(message);
    }
  };

  return {
    handleChatEvent,
    handleError,
    handleOpen,
  };
};
