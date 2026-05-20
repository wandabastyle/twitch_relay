import {
  ABR_EWMA_FAST_LIVE,
  ABR_EWMA_SLOW_LIVE,
  AUTO_LEVEL,
  BACK_BUFFER_LENGTH,
  DEFAULT_LEVEL,
  FRAG_LOADING_MAX_RETRY,
  FRAG_LOADING_TIMEOUT,
  LEVEL_LOADING_MAX_RETRY,
  LEVEL_LOADING_TIMEOUT,
  LIVE_MAX_LATENCY,
  LIVE_SYNC_DURATION,
  LIVE_SYNC_START_POS,
  MANIFEST_LOADING_MAX_RETRY,
  MANIFEST_LOADING_TIMEOUT,
  MAX_BUFFER_LENGTH,
  MAX_LIVE_SYNC_PLAYBACK_RATE,
  MAX_MAX_BUFFER_LENGTH,
  RETRY_DELAY_MS,
} from '$lib/components/watch/video-player-constants';
import type { HlsInstance, HlsStatic, HlsLevel } from '$lib/components/watch/video-player-types';
import { toObject } from '$lib/components/watch/video-player-utils';

export interface HlsEventHandlers {
  setHlsLevels: (levels: HlsLevel[]) => void;
  setCurrentPlayingLevel: (level: number) => void;
  setQualityLevel: (level: number) => void;
  userSelectedAuto: boolean;
  setUserSelectedAuto: (auto: boolean) => void;
  qualityLevel: number;
  onError: (message: string) => void;
}

export const createHandleManifestParsed =
  (setHlsLevels: (levels: HlsLevel[]) => void): ((_event: string, data: unknown) => void) =>
  (_event: string, data: unknown): void => {
    const parsed = toObject(data);
    if (!parsed || !Array.isArray(parsed.levels)) {
      setHlsLevels([]);
      return;
    }

    setHlsLevels(
      parsed.levels.filter((item: unknown): item is HlsLevel => {
        const obj = toObject(item);
        return obj !== null && typeof obj.height === 'number' && typeof obj.bitrate === 'number';
      }),
    );
  };

export const createHandleLevelSwitched =
  (
    setCurrentPlayingLevel: (level: number) => void,
    setQualityLevel: (level: number) => void,
    userSelectedAuto: boolean,
  ): ((_event: string, data: unknown) => void) =>
  (_event: string, data: unknown): void => {
    const parsed = toObject(data);
    const { level: parsedLevel } = parsed ?? {};
    const level = typeof parsedLevel === 'number' ? parsedLevel : DEFAULT_LEVEL;

    setCurrentPlayingLevel(level);
    if (userSelectedAuto) {
      setQualityLevel(AUTO_LEVEL);
    }
  };

export const createHandleHlsError =
  (onError: (message: string) => void): ((_event: string, data: unknown) => void) =>
  (_event: string, data: unknown): void => {
    const parsed = toObject(data);
    if (parsed?.fatal === true) {
      onError('Stream unavailable. The channel may be offline or not accessible.');
    }
  };

export const setupHlsInstance = (HlsClass: HlsStatic): HlsInstance => {
  const instance = new HlsClass({
    abrEwmaFastLive: ABR_EWMA_FAST_LIVE,
    abrEwmaSlowLive: ABR_EWMA_SLOW_LIVE,
    backBufferLength: BACK_BUFFER_LENGTH,
    capLevelToPlayerSize: true,
    fragLoadingMaxRetry: FRAG_LOADING_MAX_RETRY,
    fragLoadingRetryDelay: RETRY_DELAY_MS,
    fragLoadingTimeOut: FRAG_LOADING_TIMEOUT,
    levelLoadingMaxRetry: LEVEL_LOADING_MAX_RETRY,
    levelLoadingRetryDelay: RETRY_DELAY_MS,
    levelLoadingTimeOut: LEVEL_LOADING_TIMEOUT,
    liveMaxLatencyDuration: LIVE_MAX_LATENCY,
    liveSyncDuration: LIVE_SYNC_DURATION,
    lowLatencyMode: true,
    manifestLoadingMaxRetry: MANIFEST_LOADING_MAX_RETRY,
    manifestLoadingRetryDelay: RETRY_DELAY_MS,
    manifestLoadingTimeOut: MANIFEST_LOADING_TIMEOUT,
    maxBufferLength: MAX_BUFFER_LENGTH,
    maxLiveSyncPlaybackRate: MAX_LIVE_SYNC_PLAYBACK_RATE,
    maxMaxBufferLength: MAX_MAX_BUFFER_LENGTH,
    startLevel: AUTO_LEVEL,
    startPosition: LIVE_SYNC_START_POS,
  });

  instance.currentLevel = AUTO_LEVEL;
  return instance;
};

export const attachHlsEvents = (
  instance: HlsInstance,
  HlsClass: HlsStatic,
  handlers: HlsEventHandlers,
): void => {
  instance.on(HlsClass.Events.MANIFEST_PARSED, createHandleManifestParsed(handlers.setHlsLevels));
  instance.on(
    HlsClass.Events.LEVEL_SWITCHED,
    createHandleLevelSwitched(
      handlers.setCurrentPlayingLevel,
      handlers.setQualityLevel,
      handlers.userSelectedAuto,
    ),
  );
  instance.on(HlsClass.Events.ERROR, createHandleHlsError(handlers.onError));
};
