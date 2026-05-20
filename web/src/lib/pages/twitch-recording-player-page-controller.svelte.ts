import {
  createRecordingRuntime,
  type HlsInstance,
} from '$lib/pages/twitch-recording-player-runtime';
import type { RecordingRuntimeState } from '$lib/pages/twitch-recording-player-types';

const TIME_ZERO = 0;

interface RecordingPlayerController {
  readonly channelLogin: string;
  readonly filename: string;
  readonly initializePlayer: () => Promise<void>;
  readonly isLoading: boolean;
  readonly playbackError: string | null;
  readonly pushProgress: (force?: boolean) => Promise<void>;
  readonly setParams: (channelLogin: string, filename: string) => void;
  readonly setPlayerElement: (value: unknown) => void;
  readonly teardown: () => void;
}

type RuntimeState = RecordingRuntimeState;

const createRuntimeState = (): RuntimeState => {
  let channelLogin = $state('');
  let filename = $state('');
  let hlsInstance = $state<HlsInstance | null>(null);
  let isLoading = $state(true);
  let lastSavedPosition = $state(TIME_ZERO);
  let playbackError = $state<string | null>(null);
  let playerEl = $state<HTMLVideoElement | null>(null);
  let progressTimer = $state<ReturnType<typeof globalThis.setInterval> | null>(null);
  let resumeSettled = $state(false);
  let resumeTargetPosition = $state<number | null>(null);

  return {
    get channelLogin() {
      return channelLogin;
    },
    set channelLogin(value: string) {
      channelLogin = value;
    },
    get filename() {
      return filename;
    },
    set filename(value: string) {
      filename = value;
    },
    get hlsInstance() {
      return hlsInstance;
    },
    set hlsInstance(value: HlsInstance | null) {
      hlsInstance = value;
    },
    get isLoading() {
      return isLoading;
    },
    set isLoading(value: boolean) {
      isLoading = value;
    },
    get lastSavedPosition() {
      return lastSavedPosition;
    },
    set lastSavedPosition(value: number) {
      lastSavedPosition = value;
    },
    get playbackError() {
      return playbackError;
    },
    set playbackError(value: string | null) {
      playbackError = value;
    },
    get playerEl() {
      return playerEl;
    },
    set playerEl(value: HTMLVideoElement | null) {
      playerEl = value;
    },
    get progressTimer() {
      return progressTimer;
    },
    set progressTimer(value: ReturnType<typeof globalThis.setInterval> | null) {
      progressTimer = value;
    },
    get resumeSettled() {
      return resumeSettled;
    },
    set resumeSettled(value: boolean) {
      resumeSettled = value;
    },
    get resumeTargetPosition() {
      return resumeTargetPosition;
    },
    set resumeTargetPosition(value: number | null) {
      resumeTargetPosition = value;
    },
  };
};

export const createRecordingPlayerController = (): Readonly<RecordingPlayerController> => {
  const state = createRuntimeState();
  const runtime = createRecordingRuntime(state);

  const setParams = (channelLoginValue: string, filenameValue: string): void => {
    state.channelLogin = channelLoginValue;
    state.filename = filenameValue;
  };

  const setPlayerElement = (value: unknown): void => {
    state.playerEl = value instanceof HTMLVideoElement ? value : null;
  };

  return {
    get channelLogin() {
      return state.channelLogin;
    },
    get filename() {
      return state.filename;
    },
    initializePlayer: runtime.initializePlayer,
    get isLoading() {
      return state.isLoading;
    },
    get playbackError() {
      return state.playbackError;
    },
    pushProgress: runtime.pushProgress,
    setParams,
    setPlayerElement,
    teardown: runtime.teardown,
  };
};
