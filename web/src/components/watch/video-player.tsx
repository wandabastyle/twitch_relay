import {
  Maximize,
  Minimize,
  PanelRightOpen,
  PictureInPicture2,
  RectangleHorizontal,
} from 'lucide-react';
import type { ReactElement } from 'react';
import { useVideoPlayer } from '../../hooks/watch/use-video-player';
import { AUTO_LEVEL, qualityLabel } from '../../hooks/watch/video-player-utils';
import { useVideoControls } from './use-video-controls';

interface VideoPlayerProps {
  manifestUrl: string;
  onError: (message: string) => void;
  chatCollapsed?: boolean;
  onToggleChat?: () => void;
  theaterMode?: boolean;
  onToggleTheater?: () => void;
  playerHandleRef?: React.RefObject<{ enterFullscreen: () => void; toggleMute: () => void } | null>;
}

export const VideoPlayer = ({
  manifestUrl,
  onError,
  chatCollapsed,
  onToggleChat,
  theaterMode = false,
  onToggleTheater,
  playerHandleRef,
}: VideoPlayerProps): ReactElement => {
  const {
    playerRef,
    qualityLevel,
    qualityMenuOpen,
    hlsLevels,
    liveButtonIsLive,
    selectedQualityLabel,
    toggleQualityMenu,
    selectQuality,
    goLive,
  } = useVideoPlayer({ manifestUrl, onError });

  const { isFullscreen, isPip, toggleFullscreen, togglePip } = useVideoControls({
    playerHandleRef,
    playerRef,
  });

  return (
    <div className="video-shell">
      <video ref={playerRef} className="video" autoPlay controls playsInline>
        Your browser cannot play this stream format.
      </video>

      <div className="overlay-controls">
        <div className="overlay-left">
          <button
            type="button"
            className={`overlay-btn go-live-btn ${liveButtonIsLive ? 'live' : ''}`}
            disabled={liveButtonIsLive}
            onClick={goLive}
          >
            {liveButtonIsLive ? 'Live' : 'Go Live'}
          </button>
        </div>
        <div className="overlay-right">
          {onToggleTheater && (
            <button
              type="button"
              className={`overlay-btn theater-btn ${theaterMode ? 'active' : ''}`}
              onClick={onToggleTheater}
              aria-label={theaterMode ? 'Exit theater mode' : 'Enter theater mode'}
              aria-pressed={theaterMode}
            >
              <RectangleHorizontal size={20} />
            </button>
          )}
          <button
            type="button"
            className="overlay-btn pip-btn"
            onClick={togglePip}
            aria-label={isPip ? 'Exit picture-in-picture' : 'Enter picture-in-picture'}
          >
            <PictureInPicture2 size={20} />
          </button>
          <button
            type="button"
            className="overlay-btn fullscreen-btn"
            onClick={toggleFullscreen}
            aria-label={isFullscreen ? 'Exit fullscreen' : 'Enter fullscreen'}
          >
            {isFullscreen ? <Minimize size={20} /> : <Maximize size={20} />}
          </button>
          <button type="button" className="overlay-btn quality-btn" onClick={toggleQualityMenu}>
            {selectedQualityLabel}
          </button>
          {chatCollapsed === true && onToggleChat && (
            <button
              type="button"
              className="overlay-btn chat-expand-btn"
              onClick={onToggleChat}
              aria-expanded="false"
              aria-label="Expand chat"
            >
              <PanelRightOpen size={20} />
            </button>
          )}
          <div className={`quality-menu ${qualityMenuOpen ? 'open' : ''}`}>
            <button
              type="button"
              className={`quality-item ${qualityLevel === AUTO_LEVEL ? 'active' : ''}`}
              onClick={() => {
                selectQuality(AUTO_LEVEL);
              }}
            >
              Auto
            </button>
            {hlsLevels.map((level, idx) => (
              <button
                key={idx}
                type="button"
                className={`quality-item ${qualityLevel === idx ? 'active' : ''}`}
                onClick={() => {
                  selectQuality(idx);
                }}
              >
                {qualityLabel(level, idx, hlsLevels)}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
