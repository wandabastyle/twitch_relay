import { useLayoutEffect, useRef, useState, type ReactElement } from 'react';
import type { EmoteItem } from '../../api-client';
import { Chat } from './chat';
import type { VideoControlsHandle } from './use-video-controls';
import { VideoPlayer } from './video-player';

interface ChatStatus {
  available: boolean;
  connected: boolean;
  message: string;
}

interface WatchContentProps {
  availableEmotes: EmoteItem[];
  channelLogin: string;
  chatAvailable: boolean;
  chatOnly: boolean;
  currentUserDisplayName?: string;
  currentUserLogin?: string;
  handleChatStatusChange: (status: ChatStatus) => void;
  handleConnectTwitch: () => void;
  handlePlaybackError: (msg: string) => void;
  handleToggleCollapse: () => void;
  handleToggleChatOnly: () => void;
  isChatCollapsed: boolean;
  manifestUrl: string;
  onToggleTheater: () => void;
  playbackError: string | undefined;
  theaterMode: boolean;
  videoPlayerRef: React.RefObject<VideoControlsHandle | null>;
  watchError: string | undefined;
  watchLoading: boolean;
}

type ChatOnlyPlayerPhase = 'entering' | 'hidden' | 'leaving' | 'visible';

const CHAT_ONLY_TRANSITION_MS = 180;

const prefersReducedMotion = (): boolean =>
  globalThis.matchMedia('(prefers-reduced-motion: reduce)').matches;

export const WatchContent = (props: WatchContentProps): ReactElement => {
  const {
    availableEmotes,
    channelLogin,
    chatAvailable,
    chatOnly,
    currentUserDisplayName,
    currentUserLogin,
    handleChatStatusChange,
    handleConnectTwitch,
    handlePlaybackError,
    handleToggleCollapse,
    handleToggleChatOnly,
    isChatCollapsed,
    manifestUrl,
    onToggleTheater,
    playbackError,
    theaterMode,
    videoPlayerRef,
    watchError,
    watchLoading,
  } = props;
  const [playerPhase, setPlayerPhase] = useState<ChatOnlyPlayerPhase>(() =>
    chatOnly ? 'hidden' : 'visible',
  );
  const previousChatOnly = useRef(chatOnly);

  useLayoutEffect(() => {
    const settledPhase = chatOnly ? 'hidden' : 'visible';
    let timeout: ReturnType<typeof globalThis.setTimeout> | null = null;
    const chatOnlyChanged = previousChatOnly.current !== chatOnly;
    previousChatOnly.current = chatOnly;

    if (!chatOnlyChanged || prefersReducedMotion()) {
      setPlayerPhase(settledPhase);
    } else {
      const transitionPhase = chatOnly ? 'entering' : 'leaving';
      setPlayerPhase(transitionPhase);
      timeout = globalThis.setTimeout(() => {
        setPlayerPhase((phase) => (phase === transitionPhase ? settledPhase : phase));
      }, CHAT_ONLY_TRANSITION_MS);
    }

    return (): void => {
      if (timeout !== null) {
        globalThis.clearTimeout(timeout);
      }
    };
  }, [chatOnly]);

  if (watchLoading) {
    return (
      <div className="watch-loading-state">
        <p className="ui-muted">Loading watch session...</p>
      </div>
    );
  }

  if (watchError !== undefined && watchError !== '') {
    return (
      <div className="watch-loading-state">
        <p className="ui-error">{watchError}</p>
      </div>
    );
  }

  return (
    <div
      className={`watch-layout ${isChatCollapsed ? 'chat-collapsed' : ''} ${chatOnly ? 'chat-only' : ''} ${theaterMode ? 'theater' : ''}`}
    >
      <section
        aria-hidden={playerPhase !== 'visible'}
        className={`watch-player-panel player-${playerPhase}`}
        inert={playerPhase !== 'visible'}
      >
        <VideoPlayer
          chatCollapsed={isChatCollapsed}
          manifestUrl={manifestUrl}
          onError={handlePlaybackError}
          onToggleChat={handleToggleCollapse}
          onToggleTheater={onToggleTheater}
          playerHandleRef={videoPlayerRef}
          theaterMode={theaterMode}
        />

        {playbackError !== undefined && playbackError !== '' && (
          <p className="ui-error">{playbackError}</p>
        )}
      </section>

      <aside className={`watch-chat-panel ${isChatCollapsed ? 'collapsed' : ''}`}>
        {chatAvailable ? (
          <Chat
            availableEmotes={availableEmotes}
            channelLogin={channelLogin}
            chatAvailable={chatAvailable}
            currentUserDisplayName={currentUserDisplayName}
            currentUserLogin={currentUserLogin}
            chatOnly={chatOnly}
            isCollapsed={isChatCollapsed}
            onStatusChange={handleChatStatusChange}
            onToggleCollapse={handleToggleCollapse}
            onToggleChatOnly={handleToggleChatOnly}
          />
        ) : (
          <div className="chat-offline">
            <p className="ui-muted">Connect Twitch to read and send messages.</p>
            <button type="button" className="ui-nav-chip" onClick={handleConnectTwitch}>
              Connect Twitch
            </button>
          </div>
        )}
      </aside>
    </div>
  );
};
