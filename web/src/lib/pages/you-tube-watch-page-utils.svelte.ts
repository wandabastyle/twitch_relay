import {
  getYouTubeEmbedConfig,
  getYouTubeVideoMeta,
  getYouTubeVideoProgress,
  saveYouTubeVideoProgress,
} from '$lib/api-client';

const DEFAULT_END_GAP = 20;
const PERCENTAGE_MULTIPLIER = 0.05;
const RESUME_MIN_SECS = 15;
const SAVE_MIN_DELTA_SECS = 3;
const ZERO = 0;

interface SkipProgressContext {
  readonly currentTime: number;
  readonly force: boolean;
  readonly lastSaved: number;
  readonly minDelta: number;
}

interface EmbedDefaults {
  readonly autoplay: number;
  readonly quality: string;
  readonly quality_dash: string;
}

export interface VideoProgress {
  readonly completed: boolean;
  readonly position_secs: number | null;
}

export interface EmbedConfig {
  readonly defaults: Readonly<EmbedDefaults>;
  readonly referrer_policy?: string;
}

export interface VideoMeta {
  readonly duration: number;
  readonly title: string;
}

export interface WatchState {
  embedUrl: string;
  referrerPolicy: 'no-referrer' | 'strict-origin-when-cross-origin';
  isLoading: boolean;
  error: string | null;
  videoTitle: string;
  videoDuration: number | null;
  playerFrame: HTMLIFrameElement | null;
  progressTimer: ReturnType<typeof globalThis.setInterval> | null;
  lastSavedPosition: number;
}

const shouldSkipProgressSave = (ctx: Readonly<SkipProgressContext>): boolean =>
  !ctx.force && Math.abs(ctx.currentTime - ctx.lastSaved) < ctx.minDelta;

export const getEndGapSecs = (duration: number): number =>
  !Number.isFinite(duration) || duration <= ZERO
    ? DEFAULT_END_GAP
    : Math.min(DEFAULT_END_GAP, duration * PERCENTAGE_MULTIPLIER);

const calculateEndGap = (duration: number | null): number =>
  typeof duration === 'number' && duration > ZERO ? getEndGapSecs(duration) : DEFAULT_END_GAP;

const checkVideoCompleted = (
  duration: number | null,
  currentTime: number,
  endGap: number,
): boolean => typeof duration === 'number' && duration > ZERO && duration - currentTime <= endGap;

export const getEmbeddedVideoElement = (
  frame: Readonly<HTMLIFrameElement> | null,
): HTMLVideoElement | null => {
  if (frame === null) {
    return null;
  }
  try {
    const frameWindow = frame.contentWindow;
    if (frameWindow === null) {
      return null;
    }
    return frameWindow.document.querySelector('video');
  } catch {
    return null;
  }
};

export const buildEmbedUrl = (
  id: string,
  defaults: Readonly<EmbedDefaults>,
  resumeAtSecs: number | null,
): string => {
  const params = new URLSearchParams({
    autoplay: String(defaults.autoplay),
    quality: defaults.quality,
    quality_dash: defaults.quality_dash,
  });

  if (resumeAtSecs !== null && resumeAtSecs >= RESUME_MIN_SECS) {
    const resumeSeconds = String(Math.floor(resumeAtSecs));
    params.set('start', resumeSeconds);
    params.set('t', `${resumeSeconds}s`);
  }
  return `/api/youtube/embed/${encodeURIComponent(id)}?${params.toString()}`;
};

export const determineResumePosition = (
  progress: Readonly<VideoProgress>,
  duration: number,
  endGap: number,
): number | null => {
  const positionSecs = progress.position_secs;
  const maxPosition = Math.max(ZERO, duration - endGap);
  if (
    !progress.completed &&
    positionSecs !== null &&
    positionSecs >= RESUME_MIN_SECS &&
    positionSecs <= maxPosition
  ) {
    return positionSecs;
  }
  return null;
};

export const updateReferrerPolicy = (
  state: WatchState,
  referrerPolicyValue: string | undefined,
): void => {
  const DEFAULT_REFERRER_POLICY: 'no-referrer' | 'strict-origin-when-cross-origin' = 'no-referrer';
  state.referrerPolicy =
    referrerPolicyValue === 'strict-origin-when-cross-origin'
      ? 'strict-origin-when-cross-origin'
      : DEFAULT_REFERRER_POLICY;
};

export interface ProgressPayload {
  readonly currentTime: number;
  readonly duration: number | null;
  readonly videoId: string;
}

const saveProgress = async (
  state: WatchState,
  payload: Readonly<ProgressPayload>,
): Promise<void> => {
  const { currentTime, duration, videoId } = payload;
  const endGap = calculateEndGap(duration);
  const isCompleted = checkVideoCompleted(duration, currentTime, endGap);
  state.lastSavedPosition = currentTime;
  try {
    await saveYouTubeVideoProgress(videoId, {
      completed: isCompleted,
      duration_secs: duration,
      position_secs: currentTime,
    });
  } catch {
    // Keep playback uninterrupted if saving progress fails.
  }
};

export interface ProgressContext {
  readonly state: WatchState;
  readonly videoId: string;
}

interface VideoInfo {
  readonly currentTime: number;
  readonly duration: number | null;
}

const extractVideoInfo = (state: Readonly<WatchState>): VideoInfo | null => {
  const { playerFrame } = state;
  const video = getEmbeddedVideoElement(playerFrame);
  if (video === null) {
    return null;
  }
  const { currentTime, duration: videoDurationValue } = video;
  if (!Number.isFinite(currentTime) || currentTime < ZERO) {
    return null;
  }
  const duration =
    Number.isFinite(videoDurationValue) && videoDurationValue > ZERO
      ? videoDurationValue
      : state.videoDuration;
  return { currentTime, duration } as const;
};

const shouldSkipProgress = (
  videoInfo: Readonly<VideoInfo>,
  state: Readonly<WatchState>,
  force: boolean,
): boolean => {
  const skip = shouldSkipProgressSave({
    currentTime: videoInfo.currentTime,
    force,
    lastSaved: state.lastSavedPosition,
    minDelta: SAVE_MIN_DELTA_SECS,
  });
  return skip;
};

export const pushProgress = async (
  ctx: Readonly<ProgressContext>,
  force = false,
): Promise<void> => {
  const { state, videoId } = ctx;
  if (videoId === '') {
    return;
  }
  const videoInfo = extractVideoInfo(state);
  if (videoInfo === null) {
    return;
  }
  if (shouldSkipProgress(videoInfo, state, force)) {
    return;
  }
  await saveProgress(state, {
    currentTime: videoInfo.currentTime,
    duration: videoInfo.duration,
    videoId,
  });
};

export interface WatchDataResult {
  readonly config: EmbedConfig;
  readonly meta: VideoMeta;
  readonly progress: VideoProgress;
}

export const loadWatchData = async (videoId: string): Promise<WatchDataResult> => {
  const [config, meta, progress] = await Promise.all([
    getYouTubeEmbedConfig(),
    getYouTubeVideoMeta(videoId),
    getYouTubeVideoProgress(videoId),
  ]);
  return { config, meta, progress };
};
