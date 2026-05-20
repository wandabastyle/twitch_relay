import {
  createYouTubeWatchLifecycle,
  FALLBACK_VIDEO_TITLE,
  ZERO,
  type WatchController,
  type LifecycleContext,
} from './you-tube-watch-page-lifecycle.svelte';
import type {
  EmbedConfig,
  VideoMeta,
  VideoProgress,
  WatchState,
} from './you-tube-watch-page-utils.svelte';

const DEFAULT_REFERRER_POLICY: 'no-referrer' | 'strict-origin-when-cross-origin' = 'no-referrer';

export const createYouTubeWatchPageController = (videoId: string): WatchController => {
  const state = $state<WatchState>({
    embedUrl: '',
    error: null,
    isLoading: false,
    lastSavedPosition: ZERO,
    playerFrame: null,
    progressTimer: null,
    referrerPolicy: DEFAULT_REFERRER_POLICY,
    videoDuration: null,
    videoTitle: FALLBACK_VIDEO_TITLE,
  });
  const lifecycle = createYouTubeWatchLifecycle({ state, videoId } satisfies LifecycleContext);

  return {
    get embedUrl() {
      return state.embedUrl;
    },
    get error() {
      return state.error;
    },
    goBack: lifecycle.goBack,
    initialize: lifecycle.initialize,
    get isLoading() {
      return state.isLoading;
    },
    get referrerPolicy() {
      return state.referrerPolicy;
    },
    setPlayerFrame: (value: unknown): void => {
      state.playerFrame = value instanceof HTMLIFrameElement ? value : null;
    },
    stop: lifecycle.stop,
    get videoDuration() {
      return state.videoDuration;
    },
    get videoTitle() {
      return state.videoTitle;
    },
  };
};

export { ZERO, FALLBACK_VIDEO_TITLE };
export type { WatchController } from './you-tube-watch-page-lifecycle.svelte';
export type {
  WatchState,
  EmbedConfig,
  VideoMeta,
  VideoProgress,
} from './you-tube-watch-page-utils.svelte';
