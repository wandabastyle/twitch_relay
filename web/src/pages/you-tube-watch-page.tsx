import { useCallback, useEffect, useRef, useState, type ReactElement } from 'react';
import type {
  YouTubeVideoMeta,
  YouTubeWatchProgress,
  YouTubeEmbedConfig,
} from '../api-client/types';
import {
  getYouTubeEmbedConfig,
  getYouTubeVideoMeta,
  getYouTubeVideoProgress,
  saveYouTubeVideoProgress,
} from '../api-client/youtube-progress';
import { navigate } from '../router';
import { formatDuration, getEndGapSecs } from './you-tube-watch/time-utils';
import { usePageState } from './you-tube-watch/page-state';
import { getEmbeddedVideoElement } from './you-tube-watch/player-utils';

interface YouTubeWatchPageProps {
  video_id: string;
}

const ZERO = 0;
const ONE = 1;

// Interval between progress saves in milliseconds
const PROGRESS_SAVE_INTERVAL_MS = 10_000;
const RESUME_MIN_SECS = 15;
const DEFAULT_END_GAP = 20;
const SAVE_MIN_DELTA_SECS = 3;

interface WatchState {
  embedUrl: string;
  error: string | null;
  isLoading: boolean;
  lastSavedPosition: number;
  playerFrame: HTMLIFrameElement | null;
  referrerPolicy: 'no-referrer' | 'strict-origin-when-cross-origin';
  videoDuration: number | null;
  videoTitle: string;
}

const FALLBACK_VIDEO_TITLE = 'Loading...';
const DEFAULT_REFERRER_POLICY: 'no-referrer' | 'strict-origin-when-cross-origin' = 'no-referrer';

const buildEmbedUrl = (
  id: string,
  defaults: { autoplay: number; quality: string; quality_dash: string },
  resumeAtSecs: number | null,
): string => {
  const params = new URLSearchParams({
    autoplay: String(defaults.autoplay),
    quality: defaults.quality,
    quality_dash: defaults.quality_dash,
  });

  if (resumeAtSecs !== null && resumeAtSecs >= RESUME_MIN_SECS) {
    const resumeSeconds = String(Math.floor(resumeAtSecs));
    params.set('start', resumeSeconds);
    params.set('t', `${resumeSeconds}s`);
  }
  return `/api/youtube/embed/${encodeURIComponent(id)}?${params.toString()}`;
};

const determineResumePosition = (
  progress: YouTubeWatchProgress,
  duration: number,
  endGap: number,
): number | null => {
  const positionSecs = progress.position_secs;
  const maxPosition = Math.max(ZERO, duration - endGap);
  if (
    !progress.completed &&
    positionSecs !== null &&
    positionSecs >= RESUME_MIN_SECS &&
    positionSecs <= maxPosition
  ) {
    return positionSecs;
  }
  return null;
};

export const YouTubeWatchPage = ({ video_id }: YouTubeWatchPageProps): ReactElement => {
  const playerFrameRef = useRef<HTMLIFrameElement | null>(null);
  const progressTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const pageState = usePageState();
  const [state, setState] = useState<WatchState>({
    embedUrl: '',
    error: null,
    isLoading: false,
    lastSavedPosition: ZERO,
    playerFrame: null,
    referrerPolicy: DEFAULT_REFERRER_POLICY,
    videoDuration: null,
    videoTitle: FALLBACK_VIDEO_TITLE,
  });

  const goBack = useCallback((): void => {
    const returnUrl = pageState?.youtubeReturnUrl;
    if (returnUrl !== undefined && returnUrl !== '') {
      navigate(returnUrl);
    } else if (globalThis.history.length > ONE) {
      globalThis.history.back();
    } else {
      navigate('/youtube');
    }
  }, [pageState]);

  const stopProgressTimer = useCallback((): void => {
    if (progressTimerRef.current !== null) {
      clearInterval(progressTimerRef.current);
      progressTimerRef.current = null;
    }
  }, []);

  const getCurrentPosition = useCallback((): number => {
    const video = getEmbeddedVideoElement(playerFrameRef.current);
    if (video === null) {
      return ZERO;
    }
    const { currentTime } = video;
    if (!Number.isFinite(currentTime) || currentTime < ZERO) {
      return ZERO;
    }
    return currentTime;
  }, []);

  const saveProgress = useCallback(
    async (positionSecs: number, durationSecs: number | null, force = false): Promise<void> => {
      if (!force && Math.abs(positionSecs - state.lastSavedPosition) < SAVE_MIN_DELTA_SECS) {
        return;
      }

      const endGap =
        typeof durationSecs === 'number' && durationSecs > ZERO
          ? getEndGapSecs(durationSecs)
          : DEFAULT_END_GAP;
      const isCompleted =
        typeof durationSecs === 'number' &&
        durationSecs > ZERO &&
        durationSecs - positionSecs <= endGap;

      try {
        await saveYouTubeVideoProgress(video_id, {
          completed: isCompleted,
          duration_secs: durationSecs,
          position_secs: positionSecs,
        });
        setState((prev) => ({ ...prev, lastSavedPosition: positionSecs }));
      } catch {
        // Silently fail - progress saving is not critical
      }
    },
    [video_id, state.lastSavedPosition],
  );

  const pushProgress = useCallback(
    async (force = false): Promise<void> => {
      const position = getCurrentPosition();
      if (position <= ZERO) {
        return;
      }
      await saveProgress(position, state.videoDuration, force);
    },
    [getCurrentPosition, saveProgress, state.videoDuration],
  );

  const startProgressTimer = useCallback((): void => {
    stopProgressTimer();
    progressTimerRef.current = setInterval(() => {
      void pushProgress();
    }, PROGRESS_SAVE_INTERVAL_MS);
  }, [pushProgress, stopProgressTimer]);

  const initialize = useCallback(async (): Promise<void> => {
    if (!video_id) {
      setState((prev) => ({
        ...prev,
        error: 'No video ID provided',
        isLoading: false,
      }));
      return;
    }

    setState((prev) => ({ ...prev, error: null, isLoading: true }));

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

      const endGap = getEndGapSecs(videoMeta.duration);
      const resumeAt = determineResumePosition(savedProgress, videoMeta.duration, endGap);

      const embedUrl = buildEmbedUrl(
        video_id,
        embedConfig.defaults,
        resumeAt,
      );

      const referrerPolicy: 'no-referrer' | 'strict-origin-when-cross-origin' =
        embedConfig.referrer_policy === 'strict-origin-when-cross-origin'
          ? 'strict-origin-when-cross-origin'
          : DEFAULT_REFERRER_POLICY;

      setState((prev) => ({
        ...prev,
        embedUrl,
        isLoading: false,
        lastSavedPosition: resumeAt ?? prev.lastSavedPosition,
        referrerPolicy,
        videoDuration: videoMeta.duration,
        videoTitle: videoMeta.title,
      }));
    } catch (error) {
      setState((prev) => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Failed to load video',
        isLoading: false,
      }));
    }
  }, [video_id]);

  const stop = useCallback((): void => {
    stopProgressTimer();
  }, [stopProgressTimer]);

  // Initialize on mount
  useEffect(() => {
    void initialize();
    return (): void => {
      stop();
    };
  }, [initialize, stop]);

  // Start progress timer when video is ready
  useEffect(() => {
    if (state.embedUrl !== '' && !state.isLoading && state.error === null) {
      startProgressTimer();
    }
    return (): void => {
      stopProgressTimer();
    };
  }, [state.embedUrl, state.isLoading, state.error, startProgressTimer, stopProgressTimer]);

  // Save progress on beforeunload
  useEffect(() => {
    const handleBeforeUnload = (): void => {
      const position = getCurrentPosition();
      const { videoDuration } = state;
      if (videoDuration !== null && position > ZERO) {
        // Use synchronous XHR for beforeunload
        const xhr = new XMLHttpRequest();
        xhr.open('PUT', `/api/youtube/video/${encodeURIComponent(video_id)}/progress`, false);
        xhr.setRequestHeader('Content-Type', 'application/json');

        const endGap = getEndGapSecs(videoDuration);
        const isCompleted = videoDuration - position <= endGap;

        xhr.send(
          JSON.stringify({
            completed: isCompleted,
            duration_secs: videoDuration,
            position_secs: position,
          }),
        );
      }
    };

    window.addEventListener('beforeunload', handleBeforeUnload);
    return (): void => {
      window.removeEventListener('beforeunload', handleBeforeUnload);
    };
  }, [video_id, state.videoDuration, getCurrentPosition]);

  // Save progress on visibility change (tab switch)
  useEffect(() => {
    const handleVisibilityChange = (): void => {
      if (document.hidden) {
        const position = getCurrentPosition();
        if (state.videoDuration !== null && position > ZERO) {
          void saveProgress(position, state.videoDuration, true);
        }
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return (): void => {
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

      {state.error === null ? (
        state.isLoading ? (
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
        )
      ) : (
        <div className="player-wrapper">
          <div className="player error-box">
            <p className="ui-error" role="alert">
              {state.error}
            </p>
          </div>
        </div>
      )}
    </section>
  );
};
