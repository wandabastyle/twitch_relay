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

export interface RecordingRuntime {
  initializePlayer: () => Promise<void>;
  pushProgress: (force?: boolean) => Promise<void>;
  teardown: () => void;
}
