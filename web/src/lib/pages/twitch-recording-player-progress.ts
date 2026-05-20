import { getRecordingWatchProgress, saveRecordingWatchProgress } from '$lib/api-client';
import type { RecordingRuntimeState } from '$lib/pages/twitch-recording-player-types';

const DEFAULT_FALLBACK_GAP = 20;
const END_GAP_FACTOR = 0.05;
const RESUME_MIN_SECS = 15;
const SAVE_INTERVAL_MS = 10_000;
const SAVE_MIN_DELTA_SECS = 3;
const TIME_ZERO = 0;

interface ProgressValidation {
  currentTime: number;
  durationSecs?: number;
}

interface ProgressPayload {
  channel_login: string;
  completed: boolean;
  duration_secs: number | undefined;
  filename: string;
  position_secs: number;
}

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

const shouldRestoreProgress = (
  progress: Readonly<{ completed: boolean; position_secs: number | null }>,
): boolean => {
  const hasPosition = progress.position_secs !== null && progress.position_secs >= RESUME_MIN_SECS;
  return !progress.completed && hasPosition;
};

export const fetchResumeProgress = async (state: RecordingRuntimeState): Promise<void> => {
  const hasValidState = state.channelLogin !== '' && state.filename !== '';
  if (!hasValidState) {
    return;
  }
  try {
    const progress = await getRecordingWatchProgress(state.channelLogin, state.filename);
    if (shouldRestoreProgress(progress) && progress.position_secs !== null) {
      state.resumeTargetPosition = progress.position_secs;
      state.resumeSettled = false;
      state.lastSavedPosition = progress.position_secs;
    }
  } catch {
    // Ignore progress restore failures to keep playback flow available.
  }
};

export const tryResumeVideoPosition = (state: RecordingRuntimeState): void => {
  if (state.playerEl === null || state.resumeTargetPosition === null || state.resumeSettled) {
    return;
  }
  try {
    state.playerEl.currentTime = state.resumeTargetPosition;
    state.resumeSettled = true;
  } catch {
    // Ignore currentTime set failures from browser player state.
  }
};

export const canSaveProgress = (state: RecordingRuntimeState): boolean =>
  state.channelLogin !== '' && state.filename !== '' && state.playerEl !== null;

export const validateProgress = (state: RecordingRuntimeState): ProgressValidation | null => {
  if (state.playerEl === null) {
    return null;
  }
  return extractProgressValues(state.playerEl);
};

const createProgressPayload = (
  state: RecordingRuntimeState,
  currentTime: number,
  durationSecs: number | undefined,
): ProgressPayload => ({
  channel_login: state.channelLogin,
  completed: calculateCompletion(durationSecs, currentTime),
  duration_secs: durationSecs,
  filename: state.filename,
  position_secs: currentTime,
});

export const saveProgress = async (
  state: RecordingRuntimeState,
  progress: Readonly<ProgressValidation>,
): Promise<void> => {
  const { currentTime, durationSecs } = progress;
  const payload = createProgressPayload(state, currentTime, durationSecs);
  try {
    await saveRecordingWatchProgress(payload);
  } catch {
    // Ignore save failures to keep playback uninterrupted.
  }
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

export const updateAndSaveProgress = async (
  state: RecordingRuntimeState,
  result: Readonly<PushProgressResult>,
): Promise<void> => {
  if (result.currentTime === undefined) {
    return;
  }
  state.lastSavedPosition = result.currentTime;
  await saveProgress(state, { currentTime: result.currentTime, durationSecs: result.durationSecs });
};
