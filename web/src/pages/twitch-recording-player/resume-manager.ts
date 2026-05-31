import { RESUME_MIN_SECS, type RecordingRuntimeState } from './types';

export const shouldRestoreProgress = (
  progress: Readonly<{ completed: boolean; position_secs: number | null }>,
): boolean => {
  const hasPosition = progress.position_secs !== null && progress.position_secs >= RESUME_MIN_SECS;
  return !progress.completed && hasPosition;
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

export const fetchResumeProgress = async (
  state: RecordingRuntimeState,
  getRecordingWatchProgress: (channelLogin: string, filename: string) => Promise<{
    completed: boolean;
    position_secs: number | null;
  }>,
): Promise<void> => {
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
