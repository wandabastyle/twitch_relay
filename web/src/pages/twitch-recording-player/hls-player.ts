import type { HlsInstance, HlsStatic, RecordingRuntimeState } from './types';
import { FALLBACK_START_POSITION, START_LEVEL_AUTO } from './types';
import { tryResumeVideoPosition } from './resume-manager';

interface EventHandlersConfig {
  HlsClass: HlsStatic;
  hlsInstance: HlsInstance;
  resumeAt: number;
  setIsLoading: (value: boolean) => void;
  setPlaybackError: (message: string | null) => void;
  state: RecordingRuntimeState;
}

const isFatalError = (data: unknown): boolean => {
  if (typeof data !== 'object') {
    return false;
  }
  if (data === null) {
    return false;
  }
  return 'fatal' in data && data.fatal === true;
};

const createHlsConfig = (
  resumeAt: number,
): { autoStartLoad: boolean; startLevel: number; startPosition: number } => ({
  autoStartLoad: false,
  startLevel: START_LEVEL_AUTO,
  startPosition: resumeAt,
});

const setupHlsEventHandlers = (config: EventHandlersConfig): void => {
  const { hlsInstance, HlsClass, state, resumeAt, setIsLoading, setPlaybackError } = config;
  hlsInstance.on(HlsClass.Events.MANIFEST_PARSED, () => {
    setIsLoading(false);
    hlsInstance.startLoad(resumeAt);
    tryResumeVideoPosition(state);
  });
  hlsInstance.on(HlsClass.Events.ERROR, (_event, data) => {
    if (isFatalError(data)) {
      state.playbackError = 'Failed to load HLS player.';
      setPlaybackError('Failed to load HLS player.');
      setIsLoading(false);
    }
  });
};

export const teardownPlayer = (state: RecordingRuntimeState): void => {
  if (state.playerEl !== null) {
    state.playerEl.src = '';
    state.playerEl.load();
  }
};

const isNativeHlsSupported = (playerEl: HTMLVideoElement): boolean =>
  playerEl.canPlayType('application/vnd.apple.mpegurl') !== '';

const initializeNativeHls = (
  state: RecordingRuntimeState,
  playlistUrl: string,
  setIsLoading: (value: boolean) => void,
): boolean => {
  if (state.playerEl === null) {
    return false;
  }
  if (!isNativeHlsSupported(state.playerEl)) {
    return false;
  }
  state.playerEl.src = playlistUrl;
  setIsLoading(false);
  tryResumeVideoPosition(state);
  return true;
};

interface InitializeHlsConfig {
  HlsClass: HlsStatic;
  playlistUrl: string;
  setIsLoading: (value: boolean) => void;
  setPlaybackError: (message: string | null) => void;
  state: RecordingRuntimeState;
}

const initializeHlsWithConfig = (config: InitializeHlsConfig): boolean => {
  const { state, playlistUrl, HlsClass, setIsLoading, setPlaybackError } = config;
  const resumeAt = state.resumeTargetPosition ?? FALLBACK_START_POSITION;
  const hlsInstance = new HlsClass(createHlsConfig(resumeAt));
  state.hlsInstance = hlsInstance;
  setupHlsEventHandlers({
    HlsClass,
    hlsInstance,
    resumeAt,
    setIsLoading,
    setPlaybackError,
    state,
  });
  hlsInstance.loadSource(playlistUrl);
  if (state.playerEl !== null) {
    hlsInstance.attachMedia(state.playerEl);
  }
  return true;
};

interface InitializePlayerConfig {
  HlsClass: HlsStatic | null;
  playlistUrl: string;
  setIsLoading: (value: boolean) => void;
  setPlaybackError: (message: string | null) => void;
  state: RecordingRuntimeState;
}

export const initializePlayer = (config: InitializePlayerConfig): boolean => {
  const { HlsClass, playlistUrl, setIsLoading, state } = config;
  if (HlsClass === null) {
    return initializeNativeHls(state, playlistUrl, setIsLoading);
  }
  return initializeHlsWithConfig({
    HlsClass,
    playlistUrl,
    setIsLoading,
    setPlaybackError: config.setPlaybackError,
    state,
  });
};
