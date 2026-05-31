import { useCallback, useEffect, useRef, useState } from 'react';
import {
  attachPlayerEvents,
  cleanupPlayer,
  createGoLive,
  createUpdateGoLiveState,
} from '../../lib/components/watch/video-player-events';
import {
  attachHlsEvents,
  setupHlsInstance,
} from '../../lib/components/watch/video-player-hls-setup';
import type { HlsInstance, HlsLevel } from '../../lib/components/watch/video-player-types';
import {
  AUTO_LEVEL,
  ensureHlsLoaded,
  getHlsClass,
  HLS_PATH,
  qualityLabel,
  selectedQualityLabel,
  setQuality,
} from '../../lib/components/watch/video-player-utils';

export interface UseVideoPlayerReturn {
  currentPlayingLevel: number;
  hlsLevels: HlsLevel[];
  liveButtonIsLive: boolean;
  qualityLevel: number;
  qualityMenuOpen: boolean;
  selectedQualityLabel: string;
  userSelectedAuto: boolean;
  playerRef: React.RefObject<HTMLVideoElement | null>;
  toggleQualityMenu: () => void;
  selectQuality: (level: number) => void;
  goLive: () => void;
}

export interface UseVideoPlayerOptions {
  manifestUrl: string;
  onError: (message: string) => void;
}

export function useVideoPlayer(options: UseVideoPlayerOptions): UseVideoPlayerReturn {
  const { manifestUrl, onError } = options;

  const playerRef = useRef<HTMLVideoElement>(null);
  const hlsInstanceRef = useRef<HlsInstance | null>(null);

  const [currentPlayingLevel, setCurrentPlayingLevel] = useState(AUTO_LEVEL);
  const [hlsLevels, setHlsLevels] = useState<HlsLevel[]>([]);
  const [liveButtonIsLive, setLiveButtonIsLive] = useState(true);
  const [qualityLevel, setQualityLevelState] = useState(AUTO_LEVEL);
  const [qualityMenuOpen, setQualityMenuOpen] = useState(false);
  const [userSelectedAuto, setUserSelectedAuto] = useState(true);

  // Create updateGoLiveState function
  const updateGoLiveState = useCallback(() => {
    const playerEl = playerRef.current;
    const MIN_SEEKABLE_LENGTH = 0;
    const SEEKABLE_INDEX_OFFSET = 1;
    const RESUME_EXIT_LIVE_SECS = 7.5;
    const RESUME_ENTER_LIVE_SECS = 5.5;
    const ZERO = 0;

    if (!playerEl || playerEl.seekable.length <= MIN_SEEKABLE_LENGTH) {
      setLiveButtonIsLive(true);
      return;
    }

    const end = playerEl.seekable.end(playerEl.seekable.length - SEEKABLE_INDEX_OFFSET);
    const lag = Math.max(ZERO, end - playerEl.currentTime);

    if (liveButtonIsLive) {
      if (lag > RESUME_EXIT_LIVE_SECS) {
        setLiveButtonIsLive(false);
      }
    } else if (lag < RESUME_ENTER_LIVE_SECS) {
      setLiveButtonIsLive(true);
    }
  }, [liveButtonIsLive]);

  // Create goLive function
  const goLive = useCallback(() => {
    const playerEl = playerRef.current;
    const hlsInstance = hlsInstanceRef.current;
    const MIN_SEEKABLE_LENGTH = 0;
    const SEEKABLE_INDEX_OFFSET = 1;

    if (!playerEl || liveButtonIsLive) {
      return;
    }

    if (
      hlsInstance &&
      hlsInstance.liveSyncPosition !== null &&
      Number.isFinite(hlsInstance.liveSyncPosition)
    ) {
      playerEl.currentTime = hlsInstance.liveSyncPosition;
    } else if (playerEl.seekable.length > MIN_SEEKABLE_LENGTH) {
      playerEl.currentTime = playerEl.seekable.end(
        playerEl.seekable.length - SEEKABLE_INDEX_OFFSET,
      );
    }
    updateGoLiveState();
  }, [liveButtonIsLive, updateGoLiveState]);

  const handleQualityLevel = useCallback((level: number): void => {
    setQuality(level, hlsInstanceRef.current);
    setQualityLevelState(level);
    setUserSelectedAuto(level === AUTO_LEVEL);
  }, []);

  const selectQuality = useCallback(
    (level: number): void => {
      handleQualityLevel(level);
      setQualityMenuOpen(false);
    },
    [handleQualityLevel],
  );

  const toggleQualityMenu = useCallback((): void => {
    setQualityMenuOpen((prev) => !prev);
  }, []);

  // Close quality menu when clicking outside
  useEffect(() => {
    const handleDocumentClick = (event: MouseEvent): void => {
      const target = event.target as HTMLElement;
      const qualityMenu = document.querySelector('.watch-overlay-quality-menu');
      const qualityBtn = document.querySelector('.watch-overlay-btn.quality-btn');

      if (
        qualityMenuOpen &&
        qualityMenu &&
        qualityBtn &&
        !qualityMenu.contains(target) &&
        !qualityBtn.contains(target)
      ) {
        setQualityMenuOpen(false);
      }
    };

    document.addEventListener('click', handleDocumentClick);
    return () => {
      document.removeEventListener('click', handleDocumentClick);
    };
  }, [qualityMenuOpen]);

  // Setup HLS playback
  const setupHlsPlayback = useCallback(
    (HlsClass: ReturnType<typeof getHlsClass>): void => {
      if (!HlsClass || !playerRef.current) {
        return;
      }
      const instance = setupHlsInstance(HlsClass);
      hlsInstanceRef.current = instance;
      setQualityLevelState(AUTO_LEVEL);
      setCurrentPlayingLevel(AUTO_LEVEL);
      setUserSelectedAuto(true);

      attachHlsEvents(instance, HlsClass, {
        onError,
        qualityLevel,
        setCurrentPlayingLevel,
        setHlsLevels,
        setQualityLevel: setQualityLevelState,
        setUserSelectedAuto,
        userSelectedAuto,
      });
      instance.loadSource(manifestUrl);
      instance.attachMedia(playerRef.current);
    },
    [manifestUrl, onError, qualityLevel, userSelectedAuto],
  );

  const setupNativePlayback = useCallback((): void => {
    const playerEl = playerRef.current;
    if (playerEl && playerEl.canPlayType('application/vnd.apple.mpegurl')) {
      playerEl.src = manifestUrl;
    } else {
      onError('Your browser does not support HLS playback.');
    }
  }, [manifestUrl, onError]);

  const setupPlayerWithHls = useCallback(
    (HlsClass: ReturnType<typeof getHlsClass>): void => {
      if (!playerRef.current) {
        return;
      }

      if (HlsClass && HlsClass.isSupported()) {
        setupHlsPlayback(HlsClass);
      } else {
        setupNativePlayback();
      }
    },
    [setupHlsPlayback, setupNativePlayback],
  );

  // Initial setup
  useEffect(() => {
    const setupPlayer = async (): Promise<void> => {
      if (!playerRef.current) {
        return;
      }

      const hlsLoaded = await ensureHlsLoaded(HLS_PATH);
      if (!hlsLoaded) {
        onError('Failed to load HLS player.');
        return;
      }

      const HlsClass = getHlsClass();
      if (HlsClass) {
        setupPlayerWithHls(HlsClass);
      }

      attachPlayerEvents(playerRef.current, updateGoLiveState, onError);
    };

    const setup = setupPlayer();
    setup.catch(() => {
      // Ignore setup errors as they're handled via onError callback
    });

    return () => {
      cleanupPlayer(playerRef.current, updateGoLiveState, hlsInstanceRef.current);
      hlsInstanceRef.current = null;
    };
  }, [setupPlayerWithHls, updateGoLiveState, onError]);

  const qualityLabelValue = selectedQualityLabel(qualityLevel, currentPlayingLevel, hlsLevels);

  return {
    currentPlayingLevel,
    hlsLevels,
    liveButtonIsLive,
    qualityLevel,
    qualityMenuOpen,
    selectedQualityLabel: qualityLabelValue,
    userSelectedAuto,
    playerRef,
    toggleQualityMenu,
    selectQuality,
    goLive,
  };
}
