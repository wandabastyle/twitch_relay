import { PanelRightClose } from 'lucide-react';
import { useCallback, useRef, type ReactElement } from 'react';
import type { EmoteItem } from '../../api-client';
import { emoteUrl, formatUnreadMessage, readableSenderColor } from '../../hooks/watch/chat-utils';
import { useChat } from '../../hooks/watch/use-chat';
import { ChatComposer, type ChatComposerHandle } from './chat-composer';
import { EmotePicker } from './emote-picker';

const CHAT_EMPTY_LENGTH = 0;
const MESSAGE_PARTS_EMPTY_LENGTH = 0;
const UNREAD_COUNT_ZERO = 0;

interface ChatProps {
  channelLogin: string;
  chatAvailable: boolean;
  availableEmotes?: EmoteItem[];
  onStatusChange: (status: { available: boolean; connected: boolean; message: string }) => void;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
}

export const Chat = ({
  channelLogin,
  chatAvailable,
  availableEmotes = [],
  onStatusChange,
  isCollapsed,
  onToggleCollapse,
}: ChatProps): ReactElement => {
  const composerRef = useRef<ChatComposerHandle>(null);
  const {
    chatMessagesRef,
    chatMessages,
    chatConnected,
    chatStatus,
    chatSending,
    unreadChatCount,
    localEmotes,
    handleScroll,
    jumpToLatest,
    sendMessage,
  } = useChat({
    channelLogin,
    chatAvailable,
    initialEmotes: availableEmotes,
    onStatusChange,
  });

  const handleEmoteSelect = useCallback((code: string): void => {
    composerRef.current?.insertEmote(code);
  }, []);

  // Collapsed state: minimal rail indicator, hook still running
  if (isCollapsed) {
    return <div className="chat-panel collapsed" aria-hidden="true" />;
  }

  // Expanded state: full chat UI
  return (
    <div className="chat-panel">
      <div className="chat-header">
        <strong>Chat</strong>
        <span className={chatConnected ? 'status-live' : undefined}>{chatStatus}</span>
        <button
          type="button"
          className="chat-header-toggle"
          onClick={onToggleCollapse}
          aria-expanded="true"
          aria-label="Collapse chat"
        >
          <PanelRightClose aria-hidden="true" />
        </button>
      </div>

      {chatAvailable ? (
        <>
          <div
            ref={chatMessagesRef}
            className="chat-messages ui-hide-scrollbar"
            onScroll={handleScroll}
          >
            {chatMessages.length === CHAT_EMPTY_LENGTH && (
              <p className="chat-empty">Waiting for messages...</p>
            )}
            {chatMessages.map((message) => (
              <div
                key={message.id}
                className={`chat-message ${message.kind === 'notice' ? 'notice' : ''}`}
              >
                <span className="sender" style={{ color: readableSenderColor(message.sender_color) }}>
                  {message.sender_display_name}
                </span>
                <span className="content">
                  {message.parts.length > MESSAGE_PARTS_EMPTY_LENGTH
                    ? message.parts.map((part, index) =>
                        part.kind === 'emote' ? (
                          <img
                            key={`${message.id}-${index}`}
                            className="emote"
                            src={part.image_url ?? emoteUrl(part.id ?? '')}
                            alt={part.code ?? ''}
                            title={part.code}
                            loading="lazy"
                            decoding="async"
                          />
                        ) : (
                          <span key={`${message.id}-${index}`}>{part.text}</span>
                        ),
                      )
                    : message.text}
                </span>
              </div>
            ))}
          </div>
          {unreadChatCount > UNREAD_COUNT_ZERO && (
            <button type="button" className="unread-pill" onClick={jumpToLatest}>
              {formatUnreadMessage(unreadChatCount)}
            </button>
          )}
          <div className="chat-form">
            <EmotePicker availableEmotes={localEmotes} onSelect={handleEmoteSelect} />

            <ChatComposer
              ref={composerRef}
              availableEmotes={localEmotes}
              disabled={chatSending}
              onSubmit={(text) => {
                void sendMessage(text);
              }}
            />
          </div>
        </>
      ) : (
        <div className="chat-offline">
          <p className="muted">Connect Twitch to read and send messages.</p>
        </div>
      )}
    </div>
  );
};
