import type { ReactElement } from 'react';
import { useVideoPlayer } from '../../hooks/watch/use-video-player';
import { AUTO_LEVEL, qualityLabel } from '../../hooks/watch/video-player-utils';

interface VideoPlayerProps {
  manifestUrl: string;
  onError: (message: string) => void;
}

export const VideoPlayer = ({ manifestUrl, onError }: VideoPlayerProps): ReactElement => {
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
          <button type="button" className="overlay-btn quality-btn" onClick={toggleQualityMenu}>
            {selectedQualityLabel}
          </button>
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
