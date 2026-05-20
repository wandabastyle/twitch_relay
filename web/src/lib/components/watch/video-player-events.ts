import {
  MIN_SEEKABLE_LENGTH,
  RESUME_ENTER_LIVE_SECS,
  RESUME_EXIT_LIVE_SECS,
  SEEKABLE_INDEX_OFFSET,
  ZERO,
} from '$lib/components/watch/video-player-constants';
import type { HlsInstance, HlsLevel } from '$lib/components/watch/video-player-types';

export interface VideoPlayerState {
  currentPlayingLevel: number;
  hlsLevels: HlsLevel[];
  liveButtonIsLive: boolean;
  qualityLevel: number;
  userSelectedAuto: boolean;
}

export interface VideoPlayerActions {
  setCurrentPlayingLevel: (level: number) => void;
  setHlsLevels: (levels: HlsLevel[]) => void;
  setLiveButtonIsLive: (isLive: boolean) => void;
  setQualityLevel: (level: number) => void;
  setUserSelectedAuto: (auto: boolean) => void;
}

export interface VideoPlayerError {
  onError: (message: string) => void;
}

export const createUpdateGoLiveState =
  (
    getPlayerEl: () => HTMLVideoElement | null,
    getLiveButtonIsLive: () => boolean,
    setLiveButtonIsLive: (isLive: boolean) => void,
  ): (() => void) =>
  (): void => {
    const playerEl = getPlayerEl();
    const liveButtonIsLive = getLiveButtonIsLive();

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
  };

export const createGoLive =
  (
    getPlayerEl: () => HTMLVideoElement | null,
    getLiveButtonIsLive: () => boolean,
    getHlsInstance: () => HlsInstance | null,
    updateGoLiveState: () => void,
  ): (() => void) =>
  (): void => {
    const playerEl = getPlayerEl();
    const hlsInstance = getHlsInstance();

    if (!playerEl || getLiveButtonIsLive()) {
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
  };

export const cleanupPlayer = (
  playerEl: HTMLVideoElement | null,
  updateGoLiveState: () => void,
  hlsInstance: HlsInstance | null,
): void => {
  if (playerEl) {
    playerEl.removeEventListener('timeupdate', updateGoLiveState);
    playerEl.removeEventListener('loadedmetadata', updateGoLiveState);
    playerEl.removeEventListener('durationchange', updateGoLiveState);
  }

  if (hlsInstance) {
    hlsInstance.destroy();
  }
};

export const attachPlayerEvents = (
  playerEl: HTMLVideoElement | null,
  updateGoLiveState: () => void,
  onError: (message: string) => void,
): void => {
  if (!playerEl) {
    return;
  }

  playerEl.addEventListener('timeupdate', updateGoLiveState);
  playerEl.addEventListener('loadedmetadata', updateGoLiveState);
  playerEl.addEventListener('durationchange', updateGoLiveState);
  playerEl.addEventListener('error', () => {
    onError('Stream unavailable. The channel may be offline or not accessible.');
  });
};
