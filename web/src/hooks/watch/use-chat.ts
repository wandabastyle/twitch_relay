import { useCallback, useEffect, useRef, useState } from 'react';
import { getChatEmotes, type EmoteItem } from '../../api-client';
import type { ChatMessage } from '../../lib/components/watch/chat-utils.svelte';
import { useChatConnection } from './use-chat-connection';

const UNREAD_COUNT_ZERO = 0;

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

export const useChat = (options: UseChatOptions): UseChatReturn => {
  const { channelLogin, chatAvailable, initialEmotes = [], onStatusChange } = options;

  const composerRef = useRef<{ insertEmote?: (code: string) => void }>(null);
  const connection = useChatConnection();
  const { cleanupConnection, setupConnection, setChatStatus } = connection;

  const [localEmotes, setLocalEmotes] = useState<EmoteItem[]>([]);
  const [emotesLoaded, setEmotesLoaded] = useState(false);
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
    onStatusChange({
      available: chatAvailable,
      connected: connection.chatConnected,
      message: connection.chatStatus,
    });
  }, [chatAvailable, connection.chatConnected, connection.chatStatus, onStatusChange]);

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
    [channelLogin, setChatStatus],
  );

  const onComposerSelect = useCallback((code: string): void => {
    composerRef.current?.insertEmote?.(code);
  }, []);

  const insertEmote = useCallback((code: string): void => {
    composerRef.current?.insertEmote?.(code);
  }, []);

  // Setup chat on mount
  useEffect(() => {
    if (chatAvailable) {
      void setupConnection(channelLogin, chatAvailable);
      void loadEmotes();
    }

    return (): void => {
      void cleanupConnection(channelLogin);
    };
  }, [chatAvailable, channelLogin, cleanupConnection, loadEmotes, setupConnection]);

  return {
    chatConnected: connection.chatConnected,
    chatMessages: connection.chatMessages,
    chatMessagesRef: connection.chatMessagesRef,
    chatSending,
    chatStatus: connection.chatStatus,
    emotesLoaded,
    handleScroll: connection.handleScroll,
    insertEmote,
    jumpToLatest: connection.jumpToLatest,
    localEmotes,
    onComposerSelect,
    sendMessage,
    unreadChatCount: connection.unreadChatCount,
  };
};
