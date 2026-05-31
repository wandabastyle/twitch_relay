import { useCallback, useEffect, useRef, useState, type ReactElement } from 'react';
import { navigate } from '../router';
import { usePageState } from './you-tube-watch/page-state';
import { getEmbeddedVideoElement } from './you-tube-watch/player-utils';
import { useProgressManager, syncSaveProgress } from './you-tube-watch/progress-manager';
import { initializeVideoData } from './you-tube-watch/video-init';
import {
  YouTubePlayerContent,
  YouTubePlayerHeader,
} from './you-tube-watch/player-content';

interface YouTubeWatchPageProps {
  video_id: string;
}

const ZERO = 0;
const ONE = 1;

// Interval between progress saves in milliseconds
const PROGRESS_SAVE_INTERVAL_MS = 10_000;

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
    referrerPolicy: 'no-referrer',
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

  // Progress manager hook
  const progressManager = useProgressManager(
    {
      setters: {
        setLastSavedPosition: (pos: number): void => {
          setState((prev) => ({ ...prev, lastSavedPosition: pos }));
        },
      },
      state: {
        lastSavedPosition: state.lastSavedPosition,
        videoDuration: state.videoDuration,
      },
      videoId: video_id,
    },
    getCurrentPosition,
  );

  const startProgressTimer = useCallback((): void => {
    stopProgressTimer();
    progressTimerRef.current = setInterval(() => {
      void progressManager.pushProgress();
    }, PROGRESS_SAVE_INTERVAL_MS);
  }, [progressManager, stopProgressTimer]);

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
      const initResult = await initializeVideoData(video_id);

      setState((prev) => ({
        ...prev,
        embedUrl: initResult.embedUrl,
        isLoading: false,
        lastSavedPosition: initResult.resumeAt ?? prev.lastSavedPosition,
        referrerPolicy: initResult.referrerPolicy,
        videoDuration: initResult.videoDuration,
        videoTitle: initResult.videoTitle,
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
      syncSaveProgress({ position, videoDuration, videoId: video_id });
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
          void progressManager.saveProgress(position, state.videoDuration, true);
        }
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return (): void => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [state.videoDuration, getCurrentPosition, progressManager]);

  return (
    <section className="ui-page-panel ui-page-panel--wide">
      <YouTubePlayerHeader state={state} onGoBack={goBack} />
      <YouTubePlayerContent state={state} videoId={video_id} playerFrameRef={playerFrameRef} />
    </section>
  );
};
