import type { ReactElement } from 'react';
import { formatDuration } from './time-utils';

interface WatchState {
  error: string | null;
  isLoading: boolean;
  videoDuration: number | null;
  videoTitle: string;
  embedUrl: string;
  referrerPolicy: 'no-referrer' | 'strict-origin-when-cross-origin';
}

interface PlayerContentProps {
  state: WatchState;
  videoId: string;
  playerFrameRef: React.RefObject<HTMLIFrameElement | null>;
}

export const YouTubePlayerContent = ({
  state,
  videoId,
  playerFrameRef,
}: PlayerContentProps): ReactElement => {
  if (state.error !== null) {
    return (
      <div className="player-wrapper">
        <div className="player error-box">
          <p className="ui-error" role="alert">
            {state.error}
          </p>
        </div>
      </div>
    );
  }

  if (state.isLoading) {
    return (
      <div className="player-wrapper">
        <div className="player loading-box">
          <p className="ui-muted">Loading video...</p>
        </div>
      </div>
    );
  }

  if (videoId !== '' && state.embedUrl !== '') {
    return (
      <div className="player-wrapper">
        <iframe
          ref={playerFrameRef}
          className="player"
          src={state.embedUrl}
          title="Invidious video player"
          allow="autoplay; fullscreen; encrypted-media; picture-in-picture"
          allowFullScreen
          loading="eager"
          referrerPolicy={state.referrerPolicy}
        />
      </div>
    );
  }

  return (
    <div className="player-wrapper">
      <div className="player error-box">
        <p className="ui-error" role="alert">
          Unable to initialize player.
        </p>
      </div>
    </div>
  );
};

interface PlayerHeaderProps {
  state: WatchState;
  onGoBack: () => void;
}

export const YouTubePlayerHeader = ({
  state,
  onGoBack,
}: PlayerHeaderProps): ReactElement => (
  <header className="player-header">
    <div>
      <button type="button" className="ui-nav-chip" onClick={onGoBack}>
        Back to videos
      </button>
      <h1>{state.videoTitle}</h1>
      {state.videoDuration !== null && (
        <p className="subtle">Duration: {formatDuration(state.videoDuration)}</p>
      )}
    </div>
  </header>
);
