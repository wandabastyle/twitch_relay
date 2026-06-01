import {
  DEFAULT_FALLBACK_GAP,
  END_GAP_FACTOR,
  SAVE_INTERVAL_MS,
  SAVE_MIN_DELTA_SECS,
  TIME_ZERO,
  type ProgressValidation,
  type RecordingRuntimeState,
} from './types';

const getEndGapSecs = (duration: number): number => {
  const isInvalidDuration = !Number.isFinite(duration) || duration <= TIME_ZERO;
  if (isInvalidDuration) {
    return DEFAULT_FALLBACK_GAP;
  }
  return Math.min(DEFAULT_FALLBACK_GAP, duration * END_GAP_FACTOR);
};

const extractProgressValues = (playerEl: HTMLVideoElement): ProgressValidation | null => {
  const { currentTime, duration } = playerEl;
  const durationSecs = Number.isFinite(duration) && duration > TIME_ZERO ? duration : undefined;
  const isInvalidTime = !Number.isFinite(currentTime) || currentTime < TIME_ZERO;
  if (isInvalidTime) {
    return null;
  }
  return { currentTime, durationSecs };
};

const shouldSkipProgressSave = (
  currentTime: number,
  lastSaved: number,
  force: boolean,
): boolean => {
  const delta = Math.abs(currentTime - lastSaved);
  return !force && delta < SAVE_MIN_DELTA_SECS;
};

const calculateCompletion = (durationSecs: number | undefined, currentTime: number): boolean => {
  const endGap = durationSecs === undefined ? DEFAULT_FALLBACK_GAP : getEndGapSecs(durationSecs);
  return typeof durationSecs === 'number' && durationSecs - currentTime <= endGap;
};

const canSaveProgress = (state: RecordingRuntimeState): boolean =>
  state.channelLogin !== '' && state.filename !== '' && state.playerEl !== null;

const validateProgress = (state: RecordingRuntimeState): ProgressValidation | null => {
  if (state.playerEl === null) {
    return null;
  }
  return extractProgressValues(state.playerEl);
};

export interface PushProgressResult {
  shouldContinue: boolean;
  currentTime?: number;
  durationSecs?: number;
}

export const preparePushProgress = (state: RecordingRuntimeState): PushProgressResult => {
  if (!canSaveProgress(state)) {
    return { shouldContinue: false };
  }
  const progressValues = validateProgress(state);
  if (progressValues === null) {
    return { shouldContinue: false };
  }
  const { currentTime, durationSecs } = progressValues;
  return { currentTime, durationSecs, shouldContinue: true };
};

export const shouldSkipSave = (
  result: Readonly<PushProgressResult>,
  state: RecordingRuntimeState,
  force: boolean,
): boolean => {
  if (!result.shouldContinue || result.currentTime === undefined) {
    return true;
  }
  return shouldSkipProgressSave(result.currentTime, state.lastSavedPosition, force);
};

export const stopProgressTracking = (state: RecordingRuntimeState): void => {
  if (state.progressTimer !== null) {
    globalThis.clearInterval(state.progressTimer);
    state.progressTimer = null;
  }
};

export const startProgressTracking = (
  state: RecordingRuntimeState,
  pushProgressFn: (force?: boolean) => Promise<void>,
): void => {
  state.progressTimer = globalThis.setInterval(() => {
    void pushProgressFn();
  }, SAVE_INTERVAL_MS);
};

export const saveProgress = async (
  state: RecordingRuntimeState,
  progress: Readonly<ProgressValidation>,
  saveRecordingWatchProgress: (payload: {
    channel_login: string;
    completed: boolean;
    duration_secs?: number;
    filename: string;
    position_secs: number;
  }) => Promise<unknown>,
): Promise<void> => {
  const { currentTime, durationSecs } = progress;
  const payload = {
    channel_login: state.channelLogin,
    completed: calculateCompletion(durationSecs, currentTime),
    duration_secs: durationSecs,
    filename: state.filename,
    position_secs: currentTime,
  };
  try {
    await saveRecordingWatchProgress(payload);
  } catch {
    // Ignore save failures to keep playback uninterrupted.
  }
};

export const updateAndSaveProgress = async (
  state: RecordingRuntimeState,
  result: Readonly<PushProgressResult>,
  saveRecordingWatchProgress: (payload: {
    channel_login: string;
    completed: boolean;
    duration_secs?: number;
    filename: string;
    position_secs: number;
  }) => Promise<unknown>,
): Promise<void> => {
  if (result.currentTime === undefined) {
    return;
  }
  state.lastSavedPosition = result.currentTime;
  await saveProgress(
    state,
    { currentTime: result.currentTime, durationSecs: result.durationSecs },
    saveRecordingWatchProgress,
  );
};
