import { useCallback, useEffect, useRef, useState } from 'react';
import {
  attachPlayerEvents,
  cleanupPlayer,
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

export const useVideoPlayer = (options: UseVideoPlayerOptions): UseVideoPlayerReturn => {
  const { manifestUrl, onError } = options;

  const playerRef = useRef<HTMLVideoElement>(null);
  const hlsInstanceRef = useRef<HlsInstance | null>(null);

  // Playback state refs - these are accessed by event handlers
  const qualityLevelRef = useRef(AUTO_LEVEL);
  const userSelectedAutoRef = useRef(true);
  const liveButtonIsLiveRef = useRef(true);

  // React state for UI display
  const [currentPlayingLevel, setCurrentPlayingLevel] = useState(AUTO_LEVEL);
  const [hlsLevels, setHlsLevels] = useState<HlsLevel[]>([]);
  const [liveButtonIsLive, setLiveButtonIsLive] = useState(true);
  const [qualityLevel, setQualityLevelState] = useState(AUTO_LEVEL);
  const [qualityMenuOpen, setQualityMenuOpen] = useState(false);
  const [userSelectedAuto, setUserSelectedAuto] = useState(true);

  // Sync refs with state
  useEffect(() => {
    qualityLevelRef.current = qualityLevel;
  }, [qualityLevel]);

  useEffect(() => {
    userSelectedAutoRef.current = userSelectedAuto;
  }, [userSelectedAuto]);

  useEffect(() => {
    liveButtonIsLiveRef.current = liveButtonIsLive;
  }, [liveButtonIsLive]);

  // Stable event handlers that read from refs
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
      const { target } = event;
      if (!(target instanceof HTMLElement)) {
        return;
      }
      const qualityMenu = document.querySelector('.quality-menu');
      const qualityBtn = document.querySelector('.overlay-btn.quality-btn');

      if (
        qualityMenuOpen &&
        qualityMenu !== null &&
        qualityBtn !== null &&
        !qualityMenu.contains(target) &&
        !qualityBtn.contains(target)
      ) {
        setQualityMenuOpen(false);
      }
    };

    document.addEventListener('click', handleDocumentClick);
    return (): void => {
      document.removeEventListener('click', handleDocumentClick);
    };
  }, [qualityMenuOpen]);

  // HLS event handlers that use refs for mutable state
  const handleManifestParsed = useCallback((levels: HlsLevel[]): void => {
    setHlsLevels(levels);
  }, []);

  const handleLevelSwitched = useCallback(
    (level: number): void => {
      setCurrentPlayingLevel(level);
      if (userSelectedAutoRef.current) {
        setQualityLevelState(AUTO_LEVEL);
      }
    },
    [],
  );

  const handleHlsError = useCallback((): void => {
    onError('Stream unavailable. The channel may be offline or not accessible.');
  }, [onError]);

  // Update live button state
  const updateGoLiveState = useCallback((): void => {
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
    const currentLiveState = liveButtonIsLiveRef.current;

    if (currentLiveState) {
      if (lag > RESUME_EXIT_LIVE_SECS) {
        setLiveButtonIsLive(false);
      }
    } else if (lag < RESUME_ENTER_LIVE_SECS) {
      setLiveButtonIsLive(true);
    }
  }, []);

  const goLive = useCallback((): void => {
    const playerEl = playerRef.current;
    const hlsInstance = hlsInstanceRef.current;
    const MIN_SEEKABLE_LENGTH = 0;
    const SEEKABLE_INDEX_OFFSET = 1;

    if (!playerEl || liveButtonIsLiveRef.current) {
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
  }, [updateGoLiveState]);

  // Single stable effect that only depends on manifestUrl
  useEffect(() => {
    const cleanup = (): void => {
      cleanupPlayer(playerRef.current, updateGoLiveState, hlsInstanceRef.current);
      hlsInstanceRef.current = null;
    };

    if (manifestUrl !== '') {
      const setupPlayer = async (): Promise<void> => {
        const playerEl = playerRef.current;
        if (playerEl === null) {
          return;
        }

        const hlsLoaded = await ensureHlsLoaded(HLS_PATH);
        if (!hlsLoaded) {
          onError('Failed to load HLS player.');
          return;
        }

        const HlsClass = getHlsClass();
        if (HlsClass === null) {
          return;
        }

        if (HlsClass.isSupported()) {
          // Setup HLS instance
          const instance = setupHlsInstance(HlsClass);
          hlsInstanceRef.current = instance;

          // Reset state
          setQualityLevelState(AUTO_LEVEL);
          setCurrentPlayingLevel(AUTO_LEVEL);
          setUserSelectedAuto(true);
          qualityLevelRef.current = AUTO_LEVEL;
          userSelectedAutoRef.current = true;

          // Attach HLS events with stable handlers
          attachHlsEvents(instance, HlsClass, {
            onError: handleHlsError,
            qualityLevel: qualityLevelRef.current,
            setCurrentPlayingLevel: handleLevelSwitched,
            setHlsLevels: handleManifestParsed,
            setQualityLevel: setQualityLevelState,
            setUserSelectedAuto,
            userSelectedAuto: userSelectedAutoRef.current,
          });

          instance.loadSource(manifestUrl);
          instance.attachMedia(playerEl);
        } else if (playerEl.canPlayType('application/vnd.apple.mpegurl')) {
          playerEl.src = manifestUrl;
        } else {
          onError('Your browser does not support HLS playback.');
        }

        // Attach player events
        attachPlayerEvents(playerEl, updateGoLiveState, onError);
      };

      const setup = setupPlayer();
      setup.catch(() => {
        // Ignore setup errors as they're handled via onError callback
      });

      // Return cleanup function
      return cleanup;
    }

      cleanup();
      return;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
  }, [manifestUrl]);
  // Only depend on manifestUrl

  const qualityLabelValue = selectedQualityLabel(qualityLevel, currentPlayingLevel, hlsLevels);

  return {
    currentPlayingLevel,
    goLive,
    hlsLevels,
    liveButtonIsLive,
    playerRef,
    qualityLevel,
    qualityMenuOpen,
    selectQuality,
    selectedQualityLabel: qualityLabelValue,
    toggleQualityMenu,
    userSelectedAuto,
  };
}