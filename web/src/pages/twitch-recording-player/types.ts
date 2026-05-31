export interface HlsInstance {
  destroy(): void;
  loadSource(url: string): void;
  attachMedia(media: unknown): void;
  on(event: string, callback: (event: unknown, data: unknown) => void): void;
  startLoad(position?: number): void;
}

export interface HlsStatic {
  new (config: Readonly<Record<string, unknown>>): HlsInstance;
  isSupported(): boolean;
  Events: { ERROR: string; MANIFEST_PARSED: string };
}

export interface ProgressValidation {
  currentTime: number;
  durationSecs?: number;
}

export interface RecordingRuntimeState {
  channelLogin: string;
  filename: string;
  playbackError: string | null;
  isLoading: boolean;
  playerEl: HTMLVideoElement | null;
  hlsInstance: HlsInstance | null;
  progressTimer: ReturnType<typeof globalThis.setInterval> | null;
  lastSavedPosition: number;
  resumeTargetPosition: number | null;
  resumeSettled: boolean;
}

export const START_LEVEL_AUTO = -1;
export const FALLBACK_START_POSITION = -1;
export const DEFAULT_FALLBACK_GAP = 20;
export const END_GAP_FACTOR = 0.05;
export const RESUME_MIN_SECS = 15;
export const SAVE_INTERVAL_MS = 10_000;
export const SAVE_MIN_DELTA_SECS = 3;
export const TIME_ZERO = 0;
