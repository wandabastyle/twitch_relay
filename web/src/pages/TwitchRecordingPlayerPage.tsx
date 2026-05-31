import { useCallback, useEffect, useRef, useState, type ReactElement } from 'react';
import { useRouter } from '../hooks/useRouter';
import { navigate } from '../router/routes';

interface HlsInstance {
  destroy(): void;
  loadSource(url: string): void;
  attachMedia(media: unknown): void;
  on(event: string, callback: (event: unknown, data: unknown) => void): void;
  startLoad(position?: number): void;
}

interface HlsStatic {
  new (config: Readonly<Record<string, unknown>>): HlsInstance;
  isSupported(): boolean;
  Events: { ERROR: string; MANIFEST_PARSED: string };
}

const HLS_SCRIPT_PATH = '/static/hls.js';
const RESUME_WAIT_MS = 100;
const START_LEVEL_AUTO = -1;
const FALLBACK_START_POSITION = -1;

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

interface RecordingRuntimeState {
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

// HLS Loader functions
const hasHlsLoaded = (): boolean => {
  const hlsValue: unknown = Reflect.get(globalThis, 'Hls');
  if (typeof hlsValue !== 'function') {
    return false;
  }
  return (
    'isSupported' in hlsValue &&
    'Events' in hlsValue &&
    typeof hlsValue.isSupported === 'function' &&
    typeof hlsValue.Events === 'object' &&
    hlsValue.Events !== null
  );
};

const delay = async (timeoutMs: number): Promise<void> => {
  await new Promise<void>((resolve) => {
    globalThis.setTimeout(() => {
      resolve();
    }, timeoutMs);
  });
};

const waitForHlsScript = async (): Promise<boolean> => {
  await delay(RESUME_WAIT_MS);
  return hasHlsLoaded();
};

const loadHlsScript = async (): Promise<boolean> => {
  const script = document.createElement('script');
  script.src = HLS_SCRIPT_PATH;
  script.async = true;
  const loaded = await new Promise<boolean>((resolve) => {
    script.addEventListener('load', () => {
      resolve(true);
    });
    script.addEventListener('error', () => {
      resolve(false);
    });
    document.head.append(script);
  });
  return loaded && hasHlsLoaded();
};

const hasExistingScript = (): boolean =>
  document.querySelector<HTMLScriptElement>(`script[src="${HLS_SCRIPT_PATH}"]`) !== null;

const ensureHlsLoaded = async (): Promise<boolean> => {
  if (typeof globalThis === 'undefined') {
    return false;
  }
  if (hasHlsLoaded()) {
    return true;
  }

  if (hasExistingScript()) {
    return await waitForHlsScript();
  }

  return await loadHlsScript();
};

const hasPlaylist = async (playlistUrl: string): Promise<boolean> => {
  try {
    const response = await fetch(playlistUrl, { method: 'HEAD' });
    return response.ok;
  } catch {
    return false;
  }
};

const hlsPlaylistUrl = (channelLogin: string, filename: string): string => {
  const params = new URLSearchParams({ channel_login: channelLogin, filename });
  return `/api/recordings/hls-playlist?${params.toString()}`;
};

const checkPlaylist = async (
  channelLogin: string,
  filename: string,
): Promise<{ exists: true; url: string } | { exists: false }> => {
  const url = hlsPlaylistUrl(channelLogin, filename);
  const exists = await hasPlaylist(url);
  if (exists) {
    return { exists: true, url };
  }
  return { exists: false };
};

// HLS Player functions
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

const setupHlsEventHandlers = (
  hlsInstance: HlsInstance,
  HlsClass: HlsStatic,
  state: RecordingRuntimeState,
  resumeAt: number,
  setIsLoading: (value: boolean) => void,
): void => {
  hlsInstance.on(HlsClass.Events.MANIFEST_PARSED, () => {
    setIsLoading(false);
    hlsInstance.startLoad(resumeAt);
    tryResumeVideoPosition(state);
  });
  hlsInstance.on(HlsClass.Events.ERROR, (_event, data) => {
    if (isFatalError(data)) {
      state.playbackError = 'Failed to load HLS player.';
      setIsLoading(false);
    }
  });
};

const teardownPlayer = (state: RecordingRuntimeState): void => {
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

const checkHlsSupport = (): { HlsClass: HlsStatic; supported: true } | { supported: false } => {
  const hlsValue: unknown = Reflect.get(globalThis, 'Hls');
  if (!isValidHlsClass(hlsValue)) {
    return { supported: false };
  }
  if (!hlsValue.isSupported()) {
    return { supported: false };
  }
  return { HlsClass: hlsValue, supported: true };
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

const initializeHls = (
  state: RecordingRuntimeState,
  playlistUrl: string,
  setIsLoading: (value: boolean) => void,
): boolean => {
  const result = checkHlsSupport();
  if (!result.supported) {
    return initializeNativeHls(state, playlistUrl, setIsLoading);
  }
  const { HlsClass } = result;
  const resumeAt = state.resumeTargetPosition ?? FALLBACK_START_POSITION;
  const hlsInstance = new HlsClass(createHlsConfig(resumeAt));
  state.hlsInstance = hlsInstance;
  setupHlsEventHandlers(hlsInstance, HlsClass, state, resumeAt, setIsLoading);
  hlsInstance.loadSource(playlistUrl);
  if (state.playerEl !== null) {
    hlsInstance.attachMedia(state.playerEl);
  }
  return true;
};

// Progress functions
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

const tryResumeVideoPosition = (state: RecordingRuntimeState): void => {
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

const canSaveProgress = (state: RecordingRuntimeState): boolean =>
  state.channelLogin !== '' && state.filename !== '' && state.playerEl !== null;

const validateProgress = (state: RecordingRuntimeState): ProgressValidation | null => {
  if (state.playerEl === null) {
    return null;
  }
  return extractProgressValues(state.playerEl);
};

interface PushProgressResult {
  shouldContinue: boolean;
  currentTime?: number;
  durationSecs?: number;
}

const preparePushProgress = (state: RecordingRuntimeState): PushProgressResult => {
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

const shouldSkipSave = (
  result: Readonly<PushProgressResult>,
  state: RecordingRuntimeState,
  force: boolean,
): boolean => {
  if (!result.shouldContinue || result.currentTime === undefined) {
    return true;
  }
  return shouldSkipProgressSave(result.currentTime, state.lastSavedPosition, force);
};

// Import API functions
import { getRecordingWatchProgress, saveRecordingWatchProgress } from '../api-client';

// Runtime functions
const fetchResumeProgress = async (state: RecordingRuntimeState): Promise<void> => {
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

const saveProgress = async (
  state: RecordingRuntimeState,
  progress: Readonly<ProgressValidation>,
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

const stopProgressTracking = (state: RecordingRuntimeState): void => {
  if (state.progressTimer !== null) {
    globalThis.clearInterval(state.progressTimer);
    state.progressTimer = null;
  }
};

const startProgressTracking = (
  state: RecordingRuntimeState,
  pushProgressFn: (force?: boolean) => Promise<void>,
): void => {
  state.progressTimer = globalThis.setInterval(() => {
    void pushProgressFn();
  }, SAVE_INTERVAL_MS);
};

const updateAndSaveProgress = async (
  state: RecordingRuntimeState,
  result: Readonly<PushProgressResult>,
): Promise<void> => {
  if (result.currentTime === undefined) {
    return;
  }
  state.lastSavedPosition = result.currentTime;
  await saveProgress(state, { currentTime: result.currentTime, durationSecs: result.durationSecs });
};

export const TwitchRecordingPlayerPage = (): ReactElement => {
  const { page } = useRouter();

  const [isInitialized, setIsInitialized] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [channelLogin, setChannelLogin] = useState('');
  const [filename, setFilename] = useState('');

  const playerElementRef = useRef<HTMLVideoElement | null>(null);
  const stateRef = useRef<RecordingRuntimeState>({
    channelLogin: '',
    filename: '',
    playbackError: null,
    isLoading: true,
    playerEl: null,
    hlsInstance: null,
    progressTimer: null,
    lastSavedPosition: 0,
    resumeTargetPosition: null,
    resumeSettled: false,
  });

  // Initialize params from URL
  useEffect(() => {
    const channelLoginParam = (page.query?.channel_login as string) ?? '';
    const filenameParam = (page.query?.filename as string) ?? '';
    setChannelLogin(channelLoginParam);
    setFilename(filenameParam);
    stateRef.current.channelLogin = channelLoginParam;
    stateRef.current.filename = filenameParam;
  }, [page.query?.channel_login, page.query?.filename]);

  const pushProgress = useCallback(async (force = false): Promise<void> => {
    const state = stateRef.current;
    const result = preparePushProgress(state);
    if (shouldSkipSave(result, state, force)) {
      return;
    }
    await updateAndSaveProgress(state, result);
  }, []);

  const teardown = useCallback((): void => {
    void pushProgress(true);
    stopProgressTracking(stateRef.current);
    if (stateRef.current.hlsInstance !== null) {
      stateRef.current.hlsInstance.destroy();
    }
    stateRef.current.hlsInstance = null;
    teardownPlayer(stateRef.current);
  }, [pushProgress]);

  const initializePlayer = useCallback(async (): Promise<void> => {
    const state = stateRef.current;
    setPlaybackError(null);
    setIsLoading(true);
    state.isLoading = true;

    await fetchResumeProgress(state);

    const playlistResult = await checkPlaylist(state.channelLogin, state.filename);
    if (!playlistResult.exists) {
      setPlaybackError('HLS playlist not available for this recording.');
      setIsLoading(false);
      state.isLoading = false;
      return;
    }

    // Load hls.js if available, but don't block on failure - native fallback will handle it
    await ensureHlsLoaded();

    // Validate player element exists
    if (state.playerEl === null) {
      setPlaybackError('Failed to initialize video player.');
      setIsLoading(false);
      state.isLoading = false;
      return;
    }

    startProgressTracking(state, pushProgress);
    const initialized = initializeHls(state, playlistResult.url, setIsLoading);
    if (!initialized) {
      setPlaybackError('HLS is not supported in this browser.');
      setIsLoading(false);
      state.isLoading = false;
    }
  }, [pushProgress]);

  // Initialize player when refs are ready
  useEffect(() => {
    if (
      playerElementRef.current === null ||
      channelLogin === '' ||
      filename === '' ||
      isInitialized
    ) {
      return;
    }

    setIsInitialized(true);
    stateRef.current.playerEl = playerElementRef.current;
    void initializePlayer();
  }, [channelLogin, filename, isInitialized, initializePlayer]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      teardown();
    };
  }, [teardown]);

  const goBack = useCallback((): void => {
    navigate('/twitch/recordings');
  }, []);

  return (
    <section className="ui-page-panel ui-page-panel--wide">
      <header className="ui-page-header">
        <div>
          <p className="ui-page-eyebrow">Recording Playback</p>
          <h1 className="ui-page-title">{channelLogin || 'unknown channel'}</h1>
          {filename && (
            <p className="ui-page-subtle" title={filename}>
              {filename}
            </p>
          )}
        </div>
        <button type="button" className="ui-nav-chip" onClick={goBack}>
          Back to recordings
        </button>
      </header>

      {!channelLogin || !filename ? (
        <p className="ui-error">Missing recording playback parameters.</p>
      ) : (
        <>
          <div className="player-wrapper">
            {isLoading && (
              <div className="player loading">
                <p className="ui-muted">Loading player...</p>
              </div>
            )}
            <video
              ref={playerElementRef}
              className={`player ${isLoading ? 'hidden' : ''}`}
              controls
              preload="auto"
            >
              Your browser cannot play this recording format.
            </video>
          </div>
          {playbackError && (
            <p className="ui-error" role="alert">
              {playbackError}
            </p>
          )}
        </>
      )}
    </section>
  );
}
