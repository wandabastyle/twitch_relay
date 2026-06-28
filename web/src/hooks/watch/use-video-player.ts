import { useCallback, useEffect, useRef, useState } from 'react';
import { attachPlayerEvents, cleanupPlayer } from './video-player-events';
import { useHlsLifecycle } from './video-player-hls';
import type { HlsLevel } from './video-player-types';
import { AUTO_LEVEL, selectedQualityLabel, setQuality } from './video-player-utils';

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

  // Playback state refs
  const qualityLevelRef = useRef(AUTO_LEVEL);
  const userSelectedAutoRef = useRef(true);
  const liveButtonIsLiveRef = useRef(true);

  // React state for UI display
  const [liveButtonIsLive, setLiveButtonIsLive] = useState(true);
  const [qualityMenuOpen, setQualityMenuOpen] = useState(false);

  // HLS lifecycle hook
  const hlsLifecycle = useHlsLifecycle();
  const { cleanupHls, hlsInstanceRef, setupHls } = hlsLifecycle;

  // Sync refs with state
  useEffect(() => {
    qualityLevelRef.current = hlsLifecycle.state.qualityLevel;
  }, [hlsLifecycle.state.qualityLevel]);

  useEffect(() => {
    userSelectedAutoRef.current = hlsLifecycle.state.userSelectedAuto;
  }, [hlsLifecycle.state.userSelectedAuto]);

  useEffect(() => {
    liveButtonIsLiveRef.current = liveButtonIsLive;
  }, [liveButtonIsLive]);

  const handleQualityLevel = useCallback(
    (level: number): void => {
      setQuality(level, hlsInstanceRef.current);
      hlsLifecycle.actions.setQualityLevel(level);
    },
    [hlsInstanceRef, hlsLifecycle.actions],
  );

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
  }, [hlsInstanceRef, updateGoLiveState]);


  useEffect(() => {
    let cleanupFn: (() => void) | null = null;

    const cleanup = (): void => {
      cleanupPlayer(playerRef.current, updateGoLiveState, hlsInstanceRef.current);
      cleanupHls();
    };

    if (manifestUrl === '') {
      cleanup();
    } else {
      const setupPlayer = async (): Promise<void> => {
        const playerEl = playerRef.current;
        if (playerEl === null) {
          return;
        }

        await setupHls(playerEl, manifestUrl, onError);
        attachPlayerEvents(playerEl, updateGoLiveState, onError);
      };

      void setupPlayer();
      cleanupFn = cleanup;
    }

    return (): void => {
      cleanupFn?.();
    };
  }, [manifestUrl, cleanupHls, hlsInstanceRef, onError, setupHls, updateGoLiveState]);

  const qualityLabelValue = selectedQualityLabel(
    hlsLifecycle.state.qualityLevel,
    hlsLifecycle.state.currentPlayingLevel,
    hlsLifecycle.state.hlsLevels,
  );

  return {
    currentPlayingLevel: hlsLifecycle.state.currentPlayingLevel,
    goLive,
    hlsLevels: hlsLifecycle.state.hlsLevels,
    liveButtonIsLive,
    playerRef,
    qualityLevel: hlsLifecycle.state.qualityLevel,
    qualityMenuOpen,
    selectQuality,
    selectedQualityLabel: qualityLabelValue,
    toggleQualityMenu,
    userSelectedAuto: hlsLifecycle.state.userSelectedAuto,
  };
};
