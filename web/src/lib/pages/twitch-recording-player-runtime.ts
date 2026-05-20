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

const validatePlayer = (state: RecordingRuntimeState, hlsLoaded: boolean): boolean => {
  if (state.playerEl === null || !hlsLoaded) {
    state.playbackError = 'Failed to load HLS player.';
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

    const hlsLoaded = await ensureHlsLoaded();
    if (!validatePlayer(state, hlsLoaded)) {
      return;
    }

    startProgressTracking(state, pushProgress);
    initializeHls(state, playlistResult.url);
  };

  return {
    initializePlayer,
    pushProgress,
    teardown,
  };
};
