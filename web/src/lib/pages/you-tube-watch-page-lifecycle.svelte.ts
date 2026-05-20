import { navigate } from '$lib/router/router.svelte';
import {
  getEndGapSecs,
  determineResumePosition,
  buildEmbedUrl,
  updateReferrerPolicy,
  pushProgress,
  loadWatchData,
  type WatchState,
} from './you-tube-watch-page-utils.svelte';

const ERROR_LOAD_FAILED = 'Failed to load embed configuration.';
const ERROR_NO_ID = 'No video ID provided.';
const FALLBACK_VIDEO_TITLE = 'YouTube video';
const SAVE_INTERVAL_MS = 10_000;
const ZERO = 0;
const ONE = 1;

export interface WatchController {
  readonly embedUrl: string;
  readonly error: string | null;
  readonly goBack: () => void;
  readonly initialize: () => void;
  readonly isLoading: boolean;
  readonly referrerPolicy: 'no-referrer' | 'strict-origin-when-cross-origin';
  readonly setPlayerFrame: (value: unknown) => void;
  readonly stop: () => void;
  readonly videoDuration: number | null;
  readonly videoTitle: string;
}

export interface LifecycleContext {
  readonly state: WatchState;
  readonly videoId: string;
}

interface LifecycleActions {
  readonly goBack: () => void;
  readonly initialize: () => void;
  readonly stop: () => void;
}

interface HistoryState {
  readonly youtubeReturnUrl?: unknown;
}

const readYouTubeReturnUrl = (): unknown => {
  const historyState: unknown = history.state;
  if (
    historyState === null ||
    typeof historyState !== 'object' ||
    !('youtubeReturnUrl' in historyState)
  ) {
    return null;
  }
  return (historyState as HistoryState).youtubeReturnUrl ?? null;
};

const goBackWithHistory = (): void => {
  const returnUrl = readYouTubeReturnUrl();
  if (typeof globalThis !== 'undefined' && returnUrl !== null) {
    globalThis.history.back();
    return;
  }
  if (globalThis.history.length > ONE) {
    globalThis.history.back();
    return;
  }
  navigate('/youtube');
};

const createGoBack = (): (() => void) => goBackWithHistory;

const stopProgressTracking = (state: Readonly<WatchState>): void => {
  if (typeof globalThis === 'undefined') {
    return;
  }
  if (state.progressTimer !== null) {
    globalThis.clearInterval(state.progressTimer);
    (state as WatchState).progressTimer = null;
  }
};

const startProgressTracking = (state: Readonly<WatchState>, videoId: string): void => {
  if (typeof globalThis === 'undefined') {
    return;
  }
  stopProgressTracking(state);
  (state as WatchState).progressTimer = globalThis.setInterval(() => {
    void pushProgress({ state: state as WatchState, videoId });
  }, SAVE_INTERVAL_MS);
};

const removeEventListeners = (
  state: Readonly<WatchState>,
  onBeforeUnload: () => void,
  onVisibilityChange: () => void,
): void => {
  if (typeof globalThis === 'undefined') {
    return;
  }
  stopProgressTracking(state);
  globalThis.removeEventListener('beforeunload', onBeforeUnload);
  document.removeEventListener('visibilitychange', onVisibilityChange);
};

interface EventListenersConfig {
  readonly onBeforeUnload: () => void;
  readonly onVisibilityChange: () => void;
  readonly state: WatchState;
  readonly videoId: string;
}

const addEventListeners = (config: Readonly<EventListenersConfig>): void => {
  if (typeof globalThis === 'undefined') {
    return;
  }
  startProgressTracking(config.state, config.videoId);
  globalThis.addEventListener('beforeunload', config.onBeforeUnload);
  document.addEventListener('visibilitychange', config.onVisibilityChange);
};

interface EmbedConfigInput {
  readonly defaults: Readonly<{
    readonly autoplay: number;
    readonly quality: string;
    readonly quality_dash: string;
  }>;
  readonly referrer_policy?: string;
}

interface VideoMetaInput {
  readonly duration: number;
  readonly title: string;
}

interface VideoProgressInput {
  readonly completed: boolean;
  readonly position_secs: number | null;
}

interface ApplyEmbedConfigContext {
  readonly state: WatchState;
  readonly videoId: string;
}

interface EmbedConfigData {
  readonly config: Readonly<EmbedConfigInput>;
  readonly ctx: Readonly<ApplyEmbedConfigContext>;
  readonly meta: Readonly<VideoMetaInput>;
  readonly progress: Readonly<VideoProgressInput>;
}

interface EventHandlers {
  readonly onBeforeUnload: () => void;
  readonly onVisibilityChange: () => void;
}

const updateStateEmbed = (
  state: WatchState,
  videoId: string,
  config: Readonly<EmbedConfigInput>,
  resumeAt: number | null,
): void => {
  state.embedUrl = buildEmbedUrl(videoId, config.defaults, resumeAt);
  updateReferrerPolicy(state, config.referrer_policy);
};

const configureWatchState = (data: Readonly<EmbedConfigData>): number | null => {
  const { ctx, config, meta, progress } = data;
  const { state, videoId } = ctx;
  const endGap = getEndGapSecs(meta.duration);
  const resumeAt = determineResumePosition(progress, meta.duration, endGap);
  if (resumeAt !== null) {
    state.lastSavedPosition = resumeAt;
  }
  state.videoTitle = meta.title;
  state.videoDuration = meta.duration;
  updateStateEmbed(state, videoId, config, resumeAt);
  return resumeAt;
};

const createBeforeUnloadHandler =
  (state: Readonly<WatchState>, videoId: string): (() => void) =>
  (): void => {
    void pushProgress({ state: state as WatchState, videoId }, true);
  };

const createVisibilityHandler =
  (state: Readonly<WatchState>, videoId: string): (() => void) =>
  (): void => {
    if (document.visibilityState === 'hidden') {
      void pushProgress({ state: state as WatchState, videoId }, true);
    }
  };

const doApplyEmbedConfig = (
  data: Readonly<EmbedConfigData>,
  registerHandlers: (handlers: EventHandlers) => void,
): void => {
  configureWatchState(data);
  const { ctx } = data;
  registerHandlers({
    onBeforeUnload: createBeforeUnloadHandler(ctx.state, ctx.videoId),
    onVisibilityChange: createVisibilityHandler(ctx.state, ctx.videoId),
  });
};

type ApplyEmbedConfigFn = (
  config: Readonly<EmbedConfigInput>,
  meta: Readonly<VideoMetaInput>,
  progress: Readonly<VideoProgressInput>,
) => void;

const createApplyEmbedConfig =
  (ctx: Readonly<LifecycleContext>): ApplyEmbedConfigFn =>
  (config, meta, progress) => {
    doApplyEmbedConfig({ config, ctx, meta, progress }, (handlers) => {
      addEventListeners({
        onBeforeUnload: handlers.onBeforeUnload,
        onVisibilityChange: handlers.onVisibilityChange,
        state: ctx.state,
        videoId: ctx.videoId,
      });
    });
  };

const initializeWatch = (
  state: WatchState,
  videoId: string,
  applyEmbedConfigFn: ApplyEmbedConfigFn,
): void => {
  if (videoId === '') {
    state.error = ERROR_NO_ID;
    return;
  }
  state.isLoading = true;
  state.error = null;
  void loadWatchData(videoId)
    .then((result) => {
      applyEmbedConfigFn(result.config, result.meta, result.progress);
    })
    .catch((error: unknown) => {
      state.error = error instanceof Error ? error.message : ERROR_LOAD_FAILED;
    })
    .finally(() => {
      state.isLoading = false;
    });
};

export const createYouTubeWatchLifecycle = (ctx: Readonly<LifecycleContext>): LifecycleActions => {
  const { state, videoId } = ctx;
  const onBeforeUnloadRef = { value: (): void => undefined };
  const onVisibilityChangeRef = { value: (): void => undefined };
  const applyEmbedConfigFn = createApplyEmbedConfig(ctx);

  return {
    goBack: createGoBack(),
    initialize: (): void => {
      initializeWatch(state, videoId, applyEmbedConfigFn);
    },
    stop: (): void => {
      removeEventListeners(state, onBeforeUnloadRef.value, onVisibilityChangeRef.value);
    },
  };
};

export { ZERO, FALLBACK_VIDEO_TITLE };
