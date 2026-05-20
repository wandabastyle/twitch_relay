import { initializeHls, teardownPlayer } from '$lib/pages/twitch-recording-player-hls';
import { checkPlaylist, ensureHlsLoaded } from '$lib/pages/twitch-recording-player-loader';
import {
  fetchResumeProgress,
  preparePushProgress,
  shouldSkipSave,
  startProgressTracking,
  stopProgressTracking,
  updateAndSaveProgress,
} from '$lib/pages/twitch-recording-player-progress';
import type {
  RecordingRuntime,
  RecordingRuntimeState,
} from '$lib/pages/twitch-recording-player-types';

export type {
  HlsInstance,
  RecordingRuntime,
  RecordingRuntimeState,
} from '$lib/pages/twitch-recording-player-types';

const validatePlayer = (state: RecordingRuntimeState): boolean => {
  if (state.playerEl === null) {
    state.playbackError = 'Failed to initialize video player.';
    state.isLoading = false;
    return false;
  }
  return true;
};

export const createRecordingRuntime = (state: RecordingRuntimeState): RecordingRuntime => {
  const pushProgress = async (force = false): Promise<void> => {
    const result = preparePushProgress(state);
    if (shouldSkipSave(result, state, force)) {
      return;
    }
    await updateAndSaveProgress(state, result);
  };

  const teardown = (): void => {
    void pushProgress(true);
    stopProgressTracking(state);
    if (state.hlsInstance !== null) {
      state.hlsInstance.destroy();
    }
    state.hlsInstance = null;
    teardownPlayer(state);
  };

  const initializePlayer = async (): Promise<void> => {
    state.playbackError = null;
    state.isLoading = true;
    await fetchResumeProgress(state);

    const playlistResult = await checkPlaylist(state.channelLogin, state.filename);
    if (!playlistResult.exists) {
      state.playbackError = 'HLS playlist not available for this recording.';
      state.isLoading = false;
      return;
    }

    // Load hls.js if available, but don't block on failure - native fallback will handle it
    await ensureHlsLoaded();

    // Validate player element exists
    if (!validatePlayer(state)) {
      return;
    }

    startProgressTracking(state, pushProgress);
    const initialized = initializeHls(state, playlistResult.url);
    if (!initialized) {
      state.playbackError = 'HLS is not supported in this browser.';
      state.isLoading = false;
    }
  };

  return {
    initializePlayer,
    pushProgress,
    teardown,
  };
};
