import { useCallback, useEffect, useRef, useState, type ReactElement } from 'react';
import { useRouter } from '../hooks/use-router';
import { navigate } from '../router/routes';
import {
  getRecordingWatchProgress,
  saveRecordingWatchProgress,
} from '../api-client';
import { checkPlaylist } from './twitch-recording-player/playlist';
import { ensureHlsLoaded, checkHlsSupport } from './twitch-recording-player/hls-loader';
import { initializePlayer, teardownPlayer } from './twitch-recording-player/hls-player';
import type { RecordingRuntimeState } from './twitch-recording-player/types';
import { fetchResumeProgress } from './twitch-recording-player/resume-manager';
import {
  preparePushProgress,
  shouldSkipSave,
  startProgressTracking,
  stopProgressTracking,
  updateAndSaveProgress,
} from './twitch-recording-player/progress-manager';

export const TwitchRecordingPlayerPage = (): ReactElement => {
  const { page } = useRouter();

  const [isInitialized, setIsInitialized] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [channelLogin, setChannelLogin] = useState('');
  const [filename, setFilename] = useState('');
  const playerElementRef = useRef<HTMLVideoElement | null>(null);
  const stateRef = useRef<RecordingRuntimeState>({
    channelLogin: '',
    filename: '',
    hlsInstance: null,
    isLoading: true,
    lastSavedPosition: 0,
    playbackError: null,
    playerEl: null,
    progressTimer: null,
    resumeSettled: false,
    resumeTargetPosition: null,
  });

  // Initialize params from URL
  useEffect(() => {
    const channelLoginParam = page.query.channel_login ?? '';
    const filenameParam = page.query.filename ?? '';
    setChannelLogin(channelLoginParam);
    setFilename(filenameParam);
    stateRef.current.channelLogin = channelLoginParam;
    stateRef.current.filename = filenameParam;
  }, [page.query.channel_login, page.query.filename]);

  const pushProgress = useCallback(async (force = false): Promise<void> => {
    const state = stateRef.current;
    const result = preparePushProgress(state);
    if (shouldSkipSave(result, state, force)) {
      return;
    }
    await updateAndSaveProgress(state, result, saveRecordingWatchProgress);
  }, []);

  const teardown = useCallback((): void => {
    void pushProgress(true);
    stopProgressTracking(stateRef.current);
    if (stateRef.current.hlsInstance !== null) {
      stateRef.current.hlsInstance.destroy();
    }
    stateRef.current.hlsInstance = null;
    teardownPlayer(stateRef.current);
  }, [pushProgress]);

  const initializePlayerFn = useCallback(async (): Promise<void> => {
    const state = stateRef.current;
    setPlaybackError(null);
    setIsLoading(true);
    state.isLoading = true;

    await fetchResumeProgress(state, getRecordingWatchProgress);

    const playlistResult = await checkPlaylist(state.channelLogin, state.filename);
    if (!playlistResult.exists) {
      setPlaybackError('HLS playlist not available for this recording.');
      setIsLoading(false);
      state.isLoading = false;
      return;
    }

    // Load hls.js if available, but don't block on failure - native fallback will handle it
    await ensureHlsLoaded();

    const hlsResult = checkHlsSupport();

    // Validate player element exists
    if (state.playerEl === null) {
      setPlaybackError('Failed to initialize video player.');
      setIsLoading(false);
      state.isLoading = false;
      return;
    }

    startProgressTracking(state, pushProgress);
    const initialized = initializePlayer({
      HlsClass: hlsResult.supported ? hlsResult.HlsClass : null,
      playlistUrl: playlistResult.url,
      setIsLoading,
      setPlaybackError,
      state,
    });
    if (!initialized) {
      setPlaybackError('HLS is not supported in this browser.');
      setIsLoading(false);
      state.isLoading = false;
    }
  }, [pushProgress]);

  // Initialize player when refs are ready
  useEffect(() => {
    if (
      playerElementRef.current === null ||
      channelLogin === '' ||
      filename === '' ||
      isInitialized
    ) {
      return;
    }

    setIsInitialized(true);
    stateRef.current.playerEl = playerElementRef.current;
    void initializePlayerFn();
  }, [channelLogin, filename, isInitialized, initializePlayerFn]);

  // Cleanup on unmount
  useEffect((): (() => void) => (): void => {
    teardown();
  }, [teardown]);

  const goBack = useCallback((): void => {
    navigate('/twitch/recordings');
  }, []);

  return (
    <section className="ui-page-panel ui-page-panel--wide">
      <header className="ui-page-header">
        <div>
          <p className="ui-page-eyebrow">Recording Playback</p>
          <h1 className="ui-page-title">{channelLogin || 'unknown channel'}</h1>
          {filename && (
            <p className="ui-page-subtle" title={filename}>
              {filename}
            </p>
          )}
        </div>
        <button type="button" className="ui-nav-chip" onClick={goBack}>
          Back to recordings
        </button>
      </header>

      {!channelLogin || !filename ? (
        <p className="ui-error">Missing recording playback parameters.</p>
      ) : (
        <>
          <div className="player-wrapper">
            {isLoading && (
              <div className="player loading">
                <p className="ui-muted">Loading player...</p>
              </div>
            )}
            <video
              ref={playerElementRef}
              className={`player ${isLoading ? 'hidden' : ''}`}
              controls
              preload="auto"
            >
              Your browser cannot play this recording format.
            </video>
          </div>
          {playbackError !== null && playbackError !== '' && (
            <p className="ui-error" role="alert">
              {playbackError}
            </p>
          )}
        </>
      )}
    </section>
  );
};
