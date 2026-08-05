import { Copy, PanelRightClose, Reply, Clock } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement } from 'react';
import type { EmoteItem } from '../../api-client';
import {
  emoteUrl,
  formatUnreadMessage,
  readableSenderColor,
  type ChatMessage,
} from '../../hooks/watch/chat-utils';
import { useChat } from '../../hooks/watch/use-chat';
import { ChatComposer, type ChatComposerHandle } from './chat-composer';
import { EmotePicker } from './emote-picker';

const CHAT_EMPTY_LENGTH = 0;
const MESSAGE_PARTS_EMPTY_LENGTH = 0;
const UNREAD_COUNT_ZERO = 0;
const TIMESTAMP_STORAGE_KEY = 'twitch-relay-chat-timestamps';
const MILLISECONDS_PER_SECOND = 1000;
const PAD_WIDTH = 2;

const padTwo = (value: number): string => String(value).padStart(PAD_WIDTH, '0');

const formatTimestamp = (unix: number): string => {
  const date = new Date(unix * MILLISECONDS_PER_SECOND);
  return `${padTwo(date.getHours())}:${padTwo(date.getMinutes())}`;
};

const escapeRegExp = (value: string): string =>
  value.replaceAll(/[.*+?^${}()|[\]\\]/g, String.raw`\\$\u0026`);

interface MentionMatch {
  end: number;
  start: number;
}

const findMentions = (
  text: string,
  login: string | undefined,
  displayName: string | undefined,
): MentionMatch[] => {
  const EMPTY_LENGTH = 0;
  if ((login === undefined || login === '') && (displayName === undefined || displayName === '')) {
    return [];
  }

  const names: string[] = [];
  if (login !== undefined && login !== '') {
    names.push(escapeRegExp(login));
  }
  if (
    displayName !== undefined &&
    displayName !== '' &&
    displayName.toLowerCase() !== login?.toLowerCase()
  ) {
    names.push(escapeRegExp(displayName));
  }

  if (names.length === EMPTY_LENGTH) {
    return [];
  }

  const pattern = new RegExp(`(^|[^\\w@])@?(?:${names.join('|')})\\b`, 'gi');
  const matches: MentionMatch[] = [];
  let match = pattern.exec(text);
  while (match !== null) {
    const [fullMatch, prefix] = match;
    const prefixLength = prefix.length;
    const start = match.index + prefixLength;
    const end = start + fullMatch.length - prefixLength;
    matches.push({ end, start });
    match = pattern.exec(text);
  }
  return matches;
};

const renderHighlightedText = (
  text: string,
  messageId: string,
  login: string | undefined,
  displayName: string | undefined,
): React.ReactNode[] => {
  const EMPTY_LENGTH = 0;
  const matches = findMentions(text, login, displayName);
  if (matches.length === EMPTY_LENGTH) {
    return [text];
  }

  const elements: React.ReactNode[] = [];
  let lastIndex = 0;
  for (const [index, match] of matches.entries()) {
    if (match.start > lastIndex) {
      elements.push(
        <span key={`${messageId}-text-${index}-pre`}>{text.slice(lastIndex, match.start)}</span>,
      );
    }
    elements.push(
      <span key={`${messageId}-mention-${index}`} className="chat-mention">
        {text.slice(match.start, match.end)}
      </span>,
    );
    lastIndex = match.end;
  }
  if (lastIndex < text.length) {
    elements.push(<span key={`${messageId}-text-tail`}>{text.slice(lastIndex)}</span>);
  }
  return elements;
};

interface ChatProps {
  channelLogin: string;
  chatAvailable: boolean;
  availableEmotes?: EmoteItem[];
  onStatusChange: (status: { available: boolean; connected: boolean; message: string }) => void;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  currentUserLogin?: string;
  currentUserDisplayName?: string;
}

const ChatMessageActions = ({
  message,
  onCopy,
  onReply,
}: {
  message: ChatMessage;
  onCopy: (message: ChatMessage) => void;
  onReply: (message: ChatMessage) => void;
}): ReactElement => (
  <span className="chat-message-actions" role="group" aria-label="Message actions">
    <button
      type="button"
      className="chat-message-action-btn"
      onClick={() => {
        onCopy(message);
      }}
      aria-label={`Copy message from ${message.sender_display_name}`}
      title="Copy message"
    >
      <Copy aria-hidden="true" size={14} />
    </button>
    <button
      type="button"
      className="chat-message-action-btn"
      onClick={() => {
        onReply(message);
      }}
      aria-label={`Reply to ${message.sender_display_name}`}
      title="Reply"
    >
      <Reply aria-hidden="true" size={14} />
    </button>
  </span>
);

const ChatMessageItem = ({
  currentUserDisplayName,
  currentUserLogin,
  message,
  onCopy,
  onReply,
  showTimestamps,
}: {
  currentUserDisplayName?: string;
  currentUserLogin?: string;
  message: ChatMessage;
  onCopy: (message: ChatMessage) => void;
  onReply: (message: ChatMessage) => void;
  showTimestamps: boolean;
}): ReactElement => {
  const memoizedContent = useMemo(
    () =>
      message.parts.length > MESSAGE_PARTS_EMPTY_LENGTH
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
              <span key={`${message.id}-${index}`}>
                {renderHighlightedText(
                  part.text ?? '',
                  `${message.id}-${index}`,
                  currentUserLogin,
                  currentUserDisplayName,
                )}
              </span>
            ),
          )
        : renderHighlightedText(message.text, message.id, currentUserLogin, currentUserDisplayName),
    [currentUserDisplayName, currentUserLogin, message],
  );

  return (
    <div className={`chat-message ${message.kind === 'notice' ? 'notice' : ''}`}>
      {showTimestamps && (
        <span className="chat-timestamp" aria-label="Message time">
          {formatTimestamp(message.sent_at_unix)}
        </span>
      )}
      <span className="sender" style={{ color: readableSenderColor(message.sender_color) }}>
        {message.sender_display_name}
      </span>
      <span className="content">{memoizedContent}</span>
      <ChatMessageActions message={message} onCopy={onCopy} onReply={onReply} />
    </div>
  );
};

export const Chat = ({
  channelLogin,
  chatAvailable,
  availableEmotes = [],
  onStatusChange,
  isCollapsed,
  onToggleCollapse,
  currentUserLogin,
  currentUserDisplayName,
}: ChatProps): ReactElement => {
  const composerRef = useRef<ChatComposerHandle | null>(null);
  const [showTimestamps, setShowTimestamps] = useState(false);
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

  useEffect(() => {
    const saved = globalThis.localStorage.getItem(TIMESTAMP_STORAGE_KEY);
    setShowTimestamps(saved === 'true');
  }, []);

  const toggleTimestamps = useCallback((): void => {
    setShowTimestamps((prev) => {
      const next = !prev;
      globalThis.localStorage.setItem(TIMESTAMP_STORAGE_KEY, String(next));
      return next;
    });
  }, []);

  const handleEmoteSelect = useCallback((code: string): void => {
    composerRef.current?.insertEmote(code);
  }, []);

  const handleReply = useCallback((message: ChatMessage): void => {
    const composer = composerRef.current;
    if (composer === null) {
      return;
    }
    const mention = `@${message.sender_display_name} `;
    composer.insertText(mention);
    composer.focus();
  }, []);

  const handleCopy = useCallback((message: ChatMessage): void => {
    void (async (): Promise<void> => {
      try {
        await globalThis.navigator.clipboard.writeText(message.text);
      } catch {
        // Ignore clipboard errors.
      }
    })();
  }, []);

  // Collapsed state: minimal rail indicator, hook still running
  if (isCollapsed) {
    return <div className="chat-panel collapsed" aria-hidden="true" />;
  }

  // Expanded state: full chat UI
  return (
    <div className="chat-panel">
      <div className="chat-header">
        <div className="chat-header-title">
          <strong>Chat</strong>
          <span className={chatConnected ? 'status-live' : undefined}>{chatStatus}</span>
        </div>
        <div className="chat-header-actions">
          <button
            type="button"
            className={`chat-header-action-btn ${showTimestamps ? 'active' : ''}`}
            onClick={toggleTimestamps}
            aria-pressed={showTimestamps}
            aria-label="Toggle timestamps"
            title="Toggle timestamps"
          >
            <Clock aria-hidden="true" size={16} />
          </button>
          <button
            type="button"
            className="chat-header-action-btn chat-header-toggle"
            onClick={onToggleCollapse}
            aria-expanded="true"
            aria-label="Collapse chat"
            title="Collapse chat"
          >
            <PanelRightClose aria-hidden="true" size={16} />
          </button>
        </div>
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
              <ChatMessageItem
                key={message.id}
                currentUserDisplayName={currentUserDisplayName}
                currentUserLogin={currentUserLogin}
                message={message}
                onCopy={handleCopy}
                onReply={handleReply}
                showTimestamps={showTimestamps}
              />
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
