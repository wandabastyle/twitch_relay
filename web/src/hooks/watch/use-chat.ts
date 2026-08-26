import { useCallback, useEffect, useRef, useState } from 'react';
import { getChatEmotes, type EmoteItem } from '../../api-client';
import { chatErrorMessage, fetchChat } from './chat-request';
import type { ChatMessage } from './chat-utils';
import { useChatConnection } from './use-chat-connection';

const UNREAD_COUNT_ZERO = 0;
const EMPTY_EMOTES: EmoteItem[] = [];

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
  retryConnection: () => void;
  handleScroll: () => void;
  jumpToLatest: () => void;
}

export interface UseChatOptions {
  channelLogin: string;
  chatAvailable: boolean;
  initialEmotes?: EmoteItem[];
  onStatusChange: (status: ChatStatus) => void;
}

export const useChat = (options: UseChatOptions): UseChatReturn => {
  const { channelLogin, chatAvailable, initialEmotes = EMPTY_EMOTES, onStatusChange } = options;

  const connection = useChatConnection();
  const { cleanupConnection, resetChannelState, setupConnection, setChatStatus } = connection;

  const [localEmotes, setLocalEmotes] = useState<EmoteItem[]>([]);
  const [emotesLoaded, setEmotesLoaded] = useState(false);
  const [chatSending, setChatSending] = useState(false);
  const emotesLoadingRef = useRef(false);

  useEffect(() => {
    resetChannelState();
    emotesLoadingRef.current = false;
    setLocalEmotes(initialEmotes);
    setEmotesLoaded(initialEmotes.length > UNREAD_COUNT_ZERO);
  }, [channelLogin, resetChannelState]);

  // Notify status changes
  useEffect(() => {
    onStatusChange({
      available: chatAvailable,
      connected: connection.chatConnected,
      message: connection.chatStatus,
    });
  }, [chatAvailable, connection.chatConnected, connection.chatStatus, onStatusChange]);

  const loadEmotes = useCallback(async (): Promise<void> => {
    if (emotesLoaded || emotesLoadingRef.current || !channelLogin) {
      return;
    }

    emotesLoadingRef.current = true;
    try {
      const emotes = await getChatEmotes(channelLogin);
      setLocalEmotes(emotes);
      setEmotesLoaded(true);
    } catch {
      // Chat remains usable without picker emotes; a later channel setup can retry.
    } finally {
      emotesLoadingRef.current = false;
    }
  }, [channelLogin, emotesLoaded]);

  const sendMessage = useCallback(
    async (text: string): Promise<void> => {
      const trimmed = text.trim();
      if (trimmed.length === UNREAD_COUNT_ZERO) {
        return;
      }

      setChatSending(true);

      try {
        await fetchChat(
          '/api/chat/send',
          {
            body: JSON.stringify({ channel_login: channelLogin, message: trimmed }),
            credentials: 'same-origin',
            headers: { 'content-type': 'application/json' },
            method: 'POST',
          },
          'Unable to send message',
        );
        setChatStatus(`Connected to #${channelLogin}`);
      } catch (error: unknown) {
        const message = chatErrorMessage(error, 'Unable to send message');
        setChatStatus(message);
        throw error;
      } finally {
        setChatSending(false);
      }
    },
    [channelLogin, setChatStatus],
  );

  // Keep the IRC/SSE subscription lifecycle tied only to channel availability and identity.
  // Emote loading updates local state, so including it here can recreate chat.
  useEffect(() => {
    if (chatAvailable) {
      void setupConnection(channelLogin, chatAvailable);
    }

    return (): void => {
      cleanupConnection(channelLogin);
    };
  }, [chatAvailable, channelLogin, cleanupConnection, setupConnection]);

  useEffect(() => {
    if (chatAvailable) {
      void loadEmotes();
    }
  }, [chatAvailable, loadEmotes]);

  const retryConnection = useCallback((): void => {
    if (chatAvailable) {
      void setupConnection(channelLogin, chatAvailable);
    }
  }, [channelLogin, chatAvailable, setupConnection]);

  return {
    chatConnected: connection.chatConnected,
    chatMessages: connection.chatMessages,
    chatMessagesRef: connection.chatMessagesRef,
    chatSending,
    chatStatus: connection.chatStatus,
    emotesLoaded,
    handleScroll: connection.handleScroll,
    jumpToLatest: connection.jumpToLatest,
    localEmotes,
    retryConnection,
    sendMessage,
    unreadChatCount: connection.unreadChatCount,
  };
};
