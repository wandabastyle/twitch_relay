import { useCallback, useEffect, useRef, useState } from 'react';
import {
  attachHlsEvents,
  setupHlsInstance,
} from './video-player-hls-setup';
import type { HlsInstance, HlsLevel } from './video-player-types';
import {
  AUTO_LEVEL,
  ensureHlsLoaded,
  getHlsClass,
  HLS_PATH,
} from './video-player-utils';

export interface HlsState {
  currentPlayingLevel: number;
  hlsLevels: HlsLevel[];
  qualityLevel: number;
  userSelectedAuto: boolean;
}

export interface HlsActions {
  setQualityLevel: (level: number) => void;
  setUserSelectedAuto: (auto: boolean) => void;
}

export interface HlsHandlers {
  handleManifestParsed: (levels: HlsLevel[]) => void;
  handleLevelSwitched: (level: number) => void;
  handleHlsError: () => void;
}

export interface HlsSetupResult {
  hlsInstanceRef: React.RefObject<HlsInstance | null>;
  state: HlsState;
  actions: HlsActions;
  handlers: HlsHandlers;
  setupHls: (
    playerEl: HTMLVideoElement,
    manifestUrl: string,
    onError: (message: string) => void,
  ) => Promise<void>;
  cleanupHls: () => void;
}

export const useHlsLifecycle = (): HlsSetupResult => {
  const hlsInstanceRef = useRef<HlsInstance | null>(null);

  // State refs for HLS event handlers
  const qualityLevelRef = useRef(AUTO_LEVEL);
  const userSelectedAutoRef = useRef(true);

  // React state for UI display
  const [currentPlayingLevel, setCurrentPlayingLevel] = useState(AUTO_LEVEL);
  const [hlsLevels, setHlsLevels] = useState<HlsLevel[]>([]);
  const [qualityLevel, setQualityLevelState] = useState(AUTO_LEVEL);
  const [userSelectedAuto, setUserSelectedAuto] = useState(true);

  // Sync refs with state
  useEffect(() => {
    qualityLevelRef.current = qualityLevel;
  }, [qualityLevel]);

  useEffect(() => {
    userSelectedAutoRef.current = userSelectedAuto;
  }, [userSelectedAuto]);

  const setQualityLevel = useCallback((level: number): void => {
    setQualityLevelState(level);
    setUserSelectedAuto(level === AUTO_LEVEL);
  }, []);

  const setUserSelectedAutoState = useCallback((auto: boolean): void => {
    setUserSelectedAuto(auto);
  }, []);

  const handleManifestParsed = useCallback((levels: HlsLevel[]): void => {
    setHlsLevels(levels);
  }, []);

  const handleLevelSwitched = useCallback((level: number): void => {
    setCurrentPlayingLevel(level);
    if (userSelectedAutoRef.current) {
      setQualityLevelState(AUTO_LEVEL);
    }
  }, []);

  const handleHlsError = useCallback((): void => {
    // Error handled via onError callback in setup
  }, []);

  const cleanupHls = useCallback((): void => {
    if (hlsInstanceRef.current) {
      hlsInstanceRef.current.destroy();
      hlsInstanceRef.current = null;
    }
  }, []);

  const setupHls = useCallback(
    async (
      playerEl: HTMLVideoElement,
      manifestUrl: string,
      onError: (message: string) => void,
    ): Promise<void> => {
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
          onError: () => {
            onError('Stream unavailable. The channel may be offline or not accessible.');
          },
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
    },
    [handleLevelSwitched, handleManifestParsed],
  );

  return {
    actions: {
      setQualityLevel,
      setUserSelectedAuto: setUserSelectedAutoState,
    },
    cleanupHls,
    handlers: {
      handleHlsError,
      handleLevelSwitched,
      handleManifestParsed,
    },
    hlsInstanceRef,
    setupHls,
    state: {
      currentPlayingLevel,
      hlsLevels,
      qualityLevel,
      userSelectedAuto,
    },
  };
};
