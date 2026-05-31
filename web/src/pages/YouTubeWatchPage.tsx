import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import type { ReactElement } from 'react';
import type {
  YouTubeVideoMeta,
  YouTubeWatchProgress,
  YouTubeEmbedConfig,
} from '../lib/api-client/types';
import {
  getYouTubeEmbedConfig,
  getYouTubeVideoMeta,
  getYouTubeVideoProgress,
  saveYouTubeVideoProgress,
} from '../lib/api-client/youtube-progress';
import { navigate, type HistoryState } from '../router';

interface YouTubeWatchPageProps {
  video_id: string;
}

const HOURS_IN_SECONDS = 3600;
const MINUTES_IN_SECONDS = 60;
const PAD_LENGTH = 2;
const ZERO = 0;

const PROGRESS_SAVE_INTERVAL_MS = 10_000; // 10 seconds
const RESUME_THRESHOLD_SECS = 15;

interface WatchState {
  embedUrl: string;
  error: string | null;
  isLoading: boolean;
  lastSavedPosition: number;
  playerFrame: HTMLIFrameElement | null;
  progressTimer: ReturnType<typeof setInterval> | null;
  referrerPolicy: 'no-referrer' | 'strict-origin-when-cross-origin';
  videoDuration: number | null;
  videoTitle: string;
}

const FALLBACK_VIDEO_TITLE = 'Loading...';
const DEFAULT_REFERRER_POLICY: 'no-referrer' | 'strict-origin-when-cross-origin' = 'no-referrer';

function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / HOURS_IN_SECONDS);
  const minutes = Math.floor((seconds % HOURS_IN_SECONDS) / MINUTES_IN_SECONDS);
  const secs = seconds % MINUTES_IN_SECONDS;

  if (hours > ZERO) {
    return `${hours}:${minutes.toString().padStart(PAD_LENGTH, '0')}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
}

function usePageState(): HistoryState | null {
  return useSyncExternalStore(
    (callback) => {
      window.addEventListener('popstate', callback);
      return () => {
        window.removeEventListener('popstate', callback);
      };
    },
    () => {
      const rawState: unknown = window.history.state;
      if (typeof rawState !== 'object' || rawState === null) {
        return null;
      }
      const descriptor = Object.getOwnPropertyDescriptor(rawState, 'youtubeReturnUrl');
      if (descriptor !== undefined && typeof descriptor.value === 'string') {
        return { youtubeReturnUrl: descriptor.value };
      }
      return null;
    },
    () => null,
  );
}

export function YouTubeWatchPage({ video_id }: YouTubeWatchPageProps): ReactElement {
  const playerFrameRef = useRef<HTMLIFrameElement | null>(null);
  const pageState = usePageState();
  const [state, setState] = useState<WatchState>({
    embedUrl: '',
    error: null,
    isLoading: false,
    lastSavedPosition: ZERO,
    playerFrame: null,
    progressTimer: null,
    referrerPolicy: DEFAULT_REFERRER_POLICY,
    videoDuration: null,
    videoTitle: FALLBACK_VIDEO_TITLE,
  });

  const goBack = useCallback((): void => {
    const returnUrl = pageState?.youtubeReturnUrl;
    if (returnUrl !== undefined && returnUrl !== '') {
      navigate(returnUrl);
    } else {
      navigate('/youtube');
    }
  }, [pageState]);

  const stopProgressTimer = useCallback((): void => {
    if (state.progressTimer !== null) {
      clearInterval(state.progressTimer);
      setState((prev) => ({ ...prev, progressTimer: null }));
    }
  }, [state.progressTimer]);

  const saveProgress = useCallback(
    async (positionSecs: number, durationSecs: number | null): Promise<void> => {
      try {
        await saveYouTubeVideoProgress(video_id, {
          position_secs: positionSecs,
          duration_secs: durationSecs,
          completed:
            durationSecs !== null && durationSecs > ZERO && positionSecs >= durationSecs - 5,
        });
        setState((prev) => ({ ...prev, lastSavedPosition: positionSecs }));
      } catch {
        // Silently fail - progress saving is not critical
      }
    },
    [video_id],
  );

  const getCurrentPosition = useCallback((): number => {
    const frame = playerFrameRef.current;
    if (frame === null || frame.contentWindow === null) {
      return ZERO;
    }

    try {
      // Try to get current time from player via postMessage
      const message = JSON.stringify({
        event: 'listening',
        id: video_id,
      });
      frame.contentWindow.postMessage(message, '*');

      // For Invidious embed, we can try to get the position from the URL
      const currentSrc = frame.src;
      const url = new URL(currentSrc);
      const tParam = url.searchParams.get('t');
      if (tParam !== null && tParam !== '') {
        const timeMatch = /(\d+)s?/.exec(tParam);
        if (timeMatch !== null) {
          return Number.parseInt(timeMatch[1], 10);
        }
      }
    } catch {
      // Silently fail
    }
    return ZERO;
  }, [video_id]);

  const startProgressTimer = useCallback((): void => {
    stopProgressTimer();
    const timer = setInterval(() => {
      const position = getCurrentPosition();
      if (state.videoDuration !== null && position > ZERO) {
        void saveProgress(position, state.videoDuration);
      }
    }, PROGRESS_SAVE_INTERVAL_MS);
    setState((prev) => ({ ...prev, progressTimer: timer }));
  }, [state.videoDuration, getCurrentPosition, saveProgress, stopProgressTimer]);

  const resumeVideo = useCallback((positionSecs: number): void => {
    const frame = playerFrameRef.current;
    if (!frame || positionSecs < RESUME_THRESHOLD_SECS) {
      return;
    }

    try {
      // Update iframe src with time parameter
      const currentSrc = frame.src;
      if (currentSrc) {
        const url = new URL(currentSrc);
        url.searchParams.set('t', `${positionSecs}s`);
        frame.src = url.toString();
      }
    } catch {
      // Silently fail
    }
  }, []);

  const initialize = useCallback(async (): Promise<void> => {
    if (!video_id) {
      setState((prev) => ({
        ...prev,
        error: 'No video ID provided',
        isLoading: false,
      }));
      return;
    }

    setState((prev) => ({ ...prev, isLoading: true, error: null }));

    try {
      // Load embed config, metadata, and saved progress in parallel
      const [embedConfig, videoMeta, savedProgress]: [
        YouTubeEmbedConfig,
        YouTubeVideoMeta,
        YouTubeWatchProgress,
      ] = await Promise.all([
        getYouTubeEmbedConfig(),
        getYouTubeVideoMeta(video_id),
        getYouTubeVideoProgress(video_id),
      ]);

      const embedUrl = `${embedConfig.invidious_base_url}/embed/${video_id}?autoplay=${embedConfig.defaults.autoplay}&quality=${embedConfig.defaults.quality}&quality_dash=${encodeURIComponent(embedConfig.defaults.quality_dash)}`;

      setState((prev) => ({
        ...prev,
        embedUrl,
        isLoading: false,
        videoDuration: videoMeta.duration,
        videoTitle: videoMeta.title,
        referrerPolicy:
          embedConfig.referrer_policy === 'no-referrer' ||
          embedConfig.referrer_policy === 'strict-origin-when-cross-origin'
            ? embedConfig.referrer_policy
            : 'no-referrer',
      }));

      // Resume if position is >= 15 seconds and not completed
      if (
        savedProgress.position_secs !== null &&
        savedProgress.position_secs >= RESUME_THRESHOLD_SECS &&
        !savedProgress.completed
      ) {
        // Wait for iframe to load then resume
        setTimeout(() => {
          resumeVideo(savedProgress.position_secs ?? ZERO);
        }, 1000);
      }
    } catch (err) {
      setState((prev) => ({
        ...prev,
        error: err instanceof Error ? err.message : 'Failed to load video',
        isLoading: false,
      }));
    }
  }, [video_id, resumeVideo]);

  const stop = useCallback((): void => {
    stopProgressTimer();
  }, [stopProgressTimer]);

  // Initialize on mount
  useEffect(() => {
    void initialize();
    return () => {
      stop();
    };
  }, [initialize, stop]);

  // Start progress timer when video is ready
  useEffect(() => {
    if (state.embedUrl !== '' && !state.isLoading && state.error === null) {
      startProgressTimer();
    }
    return () => {
      stopProgressTimer();
    };
  }, [state.embedUrl, state.isLoading, state.error, startProgressTimer, stopProgressTimer]);

  // Save progress on beforeunload
  useEffect(() => {
    const handleBeforeUnload = (): void => {
      const position = getCurrentPosition();
      if (state.videoDuration !== null && position > ZERO) {
        // Use synchronous XHR for beforeunload
        const xhr = new XMLHttpRequest();
        xhr.open('PUT', `/api/youtube/video/${encodeURIComponent(video_id)}/progress`, false);
        xhr.setRequestHeader('Content-Type', 'application/json');
        xhr.send(
          JSON.stringify({
            position_secs: position,
            duration_secs: state.videoDuration,
            completed:
              state.videoDuration !== null &&
              state.videoDuration > ZERO &&
              position >= state.videoDuration - 5,
          }),
        );
      }
    };

    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => {
      window.removeEventListener('beforeunload', handleBeforeUnload);
    };
  }, [video_id, state.videoDuration, getCurrentPosition]);

  // Save progress on visibility change (tab switch)
  useEffect(() => {
    const handleVisibilityChange = (): void => {
      if (document.hidden) {
        const position = getCurrentPosition();
        if (state.videoDuration !== null && position > ZERO) {
          void saveProgress(position, state.videoDuration);
        }
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [state.videoDuration, getCurrentPosition, saveProgress]);

  return (
    <section className="ui-page-panel ui-page-panel--wide">
      <header className="player-header">
        <div>
          <button type="button" className="ui-nav-chip" onClick={goBack}>
            Back to videos
          </button>
          <h1>{state.videoTitle}</h1>
          {state.videoDuration !== null && (
            <p className="subtle">Duration: {formatDuration(state.videoDuration)}</p>
          )}
        </div>
      </header>

      {state.error !== null ? (
        <div className="player-wrapper">
          <div className="player error-box">
            <p className="ui-error" role="alert">
              {state.error}
            </p>
          </div>
        </div>
      ) : state.isLoading ? (
        <div className="player-wrapper">
          <div className="player loading-box">
            <p className="ui-muted">Loading video...</p>
          </div>
        </div>
      ) : video_id !== '' && state.embedUrl !== '' ? (
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
      ) : (
        <div className="player-wrapper">
          <div className="player error-box">
            <p className="ui-error" role="alert">
              Unable to initialize player.
            </p>
          </div>
        </div>
      )}
    </section>
  );
}
