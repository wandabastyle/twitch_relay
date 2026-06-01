import type { ReactElement } from 'react';
import type { EmoteItem } from '../../api-client';
import { useChat } from '../../hooks/watch/use-chat';
import { emoteUrl, formatUnreadMessage } from '../../hooks/watch/chat-utils';
import { ChatComposer } from './chat-composer';
import { EmotePicker } from './emote-picker';

const CHAT_EMPTY_LENGTH = 0;
const MESSAGE_PARTS_EMPTY_LENGTH = 0;
const UNREAD_COUNT_ZERO = 0;

interface ChatProps {
  channelLogin: string;
  chatAvailable: boolean;
  availableEmotes?: EmoteItem[];
  onStatusChange: (status: { available: boolean; connected: boolean; message: string }) => void;
}

export const Chat = ({
  channelLogin,
  chatAvailable,
  availableEmotes = [],
  onStatusChange,
}: ChatProps): ReactElement => {
  const {
    chatMessagesRef,
    chatMessages,
    chatConnected,
    chatStatus,
    chatSending,
    unreadChatCount,
    localEmotes,
    emotesLoaded: _emotesLoaded,
    handleScroll,
    jumpToLatest,
    sendMessage,
    onComposerSelect,
  } = useChat({
    channelLogin,
    chatAvailable,
    initialEmotes: availableEmotes,
    onStatusChange,
  });

  return (
    <div className="chat-panel">
      <div className="chat-header">
        <strong>Chat</strong>
        <span className={chatConnected ? 'status-live' : undefined}>{chatStatus}</span>
        {unreadChatCount > UNREAD_COUNT_ZERO && (
          <button type="button" className="unread-badge" onClick={jumpToLatest}>
            {formatUnreadMessage(unreadChatCount)}
          </button>
        )}
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
                <span className="sender" style={{ color: message.sender_color ?? undefined }}>
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
          <div className="chat-form">
            <EmotePicker availableEmotes={localEmotes} onSelect={onComposerSelect} />

            <ChatComposer
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
