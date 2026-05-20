import { tryResumeVideoPosition } from '$lib/pages/twitch-recording-player-progress';
import type {
  HlsInstance,
  HlsStatic,
  RecordingRuntimeState,
} from '$lib/pages/twitch-recording-player-types';

const START_LEVEL_AUTO = -1;
const FALLBACK_START_POSITION = -1;

interface HlsConfig extends Readonly<Record<string, unknown>> {
  autoStartLoad: boolean;
  startLevel: number;
  startPosition: number;
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

const createHlsConfig = (resumeAt: number): HlsConfig => ({
  autoStartLoad: false,
  startLevel: START_LEVEL_AUTO,
  startPosition: resumeAt,
});

export const setupHlsEventHandlers = (
  hlsInstance: HlsInstance,
  HlsClass: HlsStatic,
  state: RecordingRuntimeState,
): void => {
  hlsInstance.on(HlsClass.Events.MANIFEST_PARSED, () => {
    state.isLoading = false;
    tryResumeVideoPosition(state);
  });
  hlsInstance.on(HlsClass.Events.ERROR, (_event, data) => {
    if (isFatalError(data)) {
      state.playbackError = 'Failed to load HLS player.';
      state.isLoading = false;
    }
  });
};

export const teardownPlayer = (state: RecordingRuntimeState): void => {
  if (state.playerEl !== null) {
    state.playerEl.src = '';
    state.playerEl.load();
  }
};

const isValidHlsClass = (value: unknown): value is HlsStatic => {
  if (typeof value !== 'function') {
    return false;
  }
  if (!('isSupported' in value) || !('Events' in value)) {
    return false;
  }
  const typed = value as { isSupported: unknown; Events: unknown };
  if (typeof typed.isSupported !== 'function') {
    return false;
  }
  if (typeof typed.Events !== 'object' || typed.Events === null) {
    return false;
  }
  return true;
};

export const checkHlsSupport = ():
  | { HlsClass: HlsStatic; supported: true }
  | { supported: false } => {
  const hlsValue: unknown = Reflect.get(globalThis, 'Hls');
  if (!isValidHlsClass(hlsValue)) {
    return { supported: false };
  }
  if (!hlsValue.isSupported()) {
    return { supported: false };
  }
  return { HlsClass: hlsValue, supported: true };
};

export const initializeHls = (state: RecordingRuntimeState, playlistUrl: string): void => {
  const result = checkHlsSupport();
  if (!result.supported) {
    return;
  }
  const { HlsClass } = result;
  const resumeAt = state.resumeTargetPosition ?? FALLBACK_START_POSITION;
  const hlsInstance = new HlsClass(createHlsConfig(resumeAt));
  state.hlsInstance = hlsInstance;
  setupHlsEventHandlers(hlsInstance, HlsClass, state);
  hlsInstance.loadSource(playlistUrl);
  if (state.playerEl !== null) {
    hlsInstance.attachMedia(state.playerEl);
  }
};
