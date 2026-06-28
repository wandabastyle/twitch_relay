import type { EmoteItem } from '../../api-client';
import { Chat } from './chat';
import { VideoPlayer } from './video-player';
import type { VideoControlsHandle } from './use-video-controls';
import type { ReactElement } from 'react';

interface ChatStatus {
  available: boolean;
  connected: boolean;
  message: string;
}

interface WatchContentProps {
  availableEmotes: EmoteItem[];
  channelLogin: string;
  chatAvailable: boolean;
  currentUserDisplayName?: string;
  currentUserLogin?: string;
  handleChatStatusChange: (status: ChatStatus) => void;
  handleConnectTwitch: () => void;
  handlePlaybackError: (msg: string) => void;
  handleToggleCollapse: () => void;
  isChatCollapsed: boolean;
  manifestUrl: string;
  onToggleTheater: () => void;
  playbackError: string | undefined;
  theaterMode: boolean;
  videoPlayerRef: React.RefObject<VideoControlsHandle | null>;
  watchError: string | undefined;
  watchLoading: boolean;
}

export const WatchContent = (props: WatchContentProps): ReactElement => {
  const {
    availableEmotes,
    channelLogin,
    chatAvailable,
    currentUserDisplayName,
    currentUserLogin,
    handleChatStatusChange,
    handleConnectTwitch,
    handlePlaybackError,
    handleToggleCollapse,
    isChatCollapsed,
    manifestUrl,
    onToggleTheater,
    playbackError,
    theaterMode,
    videoPlayerRef,
    watchError,
    watchLoading,
  } = props;

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
    <div className={`watch-layout ${isChatCollapsed ? 'chat-collapsed' : ''} ${theaterMode ? 'theater' : ''}`}>
      <section className="watch-player-panel">
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
            isCollapsed={isChatCollapsed}
            onStatusChange={handleChatStatusChange}
            onToggleCollapse={handleToggleCollapse}
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
