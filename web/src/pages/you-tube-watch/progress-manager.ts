import { useCallback } from 'react';
import { saveYouTubeVideoProgress } from '../../api-client/youtube-progress';
import { getEndGapSecs } from './time-utils';

const ZERO = 0;
const DEFAULT_END_GAP = 20;
const SAVE_MIN_DELTA_SECS = 3;

export interface ProgressManagerState {
  lastSavedPosition: number;
  videoDuration: number | null;
}

export interface ProgressManagerSetters {
  setLastSavedPosition: (position: number) => void;
}

export interface ProgressManagerDeps {
  videoId: string;
  state: ProgressManagerState;
  setters: ProgressManagerSetters;
}

export interface UseProgressManagerReturn {
  saveProgress: (
    positionSecs: number,
    durationSecs: number | null,
    force?: boolean,
  ) => Promise<void>;
  pushProgress: (force?: boolean) => Promise<void>;
  getCurrentPosition: () => number;
}

export const useProgressManager = (
  deps: ProgressManagerDeps,
  getCurrentPosition: () => number,
): UseProgressManagerReturn => {
  const { videoId, state, setters } = deps;

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
        await saveYouTubeVideoProgress(videoId, {
          completed: isCompleted,
          duration_secs: durationSecs,
          position_secs: positionSecs,
        });
        setters.setLastSavedPosition(positionSecs);
      } catch {
        // Silently fail - progress saving is not critical
      }
    },
    [videoId, state.lastSavedPosition, setters],
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

  return {
    getCurrentPosition,
    pushProgress,
    saveProgress,
  };
};

export interface CreateProgressManagerReturn {
  saveProgress: (
    positionSecs: number,
    durationSecs: number | null,
    force?: boolean,
  ) => Promise<void>;
  pushProgress: (force?: boolean) => Promise<void>;
}

export const createProgressManager = (
  videoId: string,
  getPositionFn: () => number,
  state: { lastSavedPosition: number; videoDuration: number | null },
  setLastSavedPosition: (pos: number) => void,
): CreateProgressManagerReturn => {
  const saveProgress = async (
    positionSecs: number,
    durationSecs: number | null,
    force = false,
  ): Promise<void> => {
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
      await saveYouTubeVideoProgress(videoId, {
        completed: isCompleted,
        duration_secs: durationSecs,
        position_secs: positionSecs,
      });
      setLastSavedPosition(positionSecs);
    } catch {
      // Silently fail - progress saving is not critical
    }
  };

  const pushProgress = async (force = false): Promise<void> => {
    const position = getPositionFn();
    if (position <= ZERO) {
      return;
    }
    await saveProgress(position, state.videoDuration, force);
  };

  return {
    pushProgress,
    saveProgress,
  };
};

export interface SyncSaveProgressOptions {
  position: number;
  videoDuration: number | null;
  videoId: string;
}

export const syncSaveProgress = (options: SyncSaveProgressOptions): void => {
  const { position, videoDuration, videoId } = options;

  if (videoDuration === null || position <= ZERO) {
    return;
  }

  const endGap = getEndGapSecs(videoDuration);
  const isCompleted = videoDuration - position <= endGap;

  const xhr = new XMLHttpRequest();
  xhr.open('PUT', `/api/youtube/video/${encodeURIComponent(videoId)}/progress`, false);
  xhr.setRequestHeader('Content-Type', 'application/json');
  xhr.send(
    JSON.stringify({
      completed: isCompleted,
      duration_secs: videoDuration,
      position_secs: position,
    }),
  );
};
