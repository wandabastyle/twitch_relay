<script lang="ts">
  import { onDestroy } from 'svelte';

  import { getRecordingWatchProgress, saveRecordingWatchProgress } from '$lib/api-client';
  import { navigate, page } from '$lib/router/router.svelte';

  // Hls.js types (Hls is loaded dynamically from /static/hls.js)
  interface HlsInstance {
    destroy(): void;
    loadSource(url: string): void;
    attachMedia(media: HTMLVideoElement): void;
    on(event: string, callback: (event: unknown, data: unknown) => void): void;
  }

  interface HlsStatic {
    new (config: Record<string, unknown>): HlsInstance;
    isSupported(): boolean;
    Events: {
      MANIFEST_PARSED: string;
      ERROR: string;
    };
  }

  // HLS Configuration Constants
  const ABR_FAST_LIVE = 3;
  const ABR_SLOW_LIVE = 9;
  const DEFAULT_FALLBACK_GAP = 20;
  const HLS_ATTEMPTS = 50;
  const HLS_ATTEMPT_DELAY_MS = 100;
  const HLS_LOAD_TIMEOUT_MS = 3000;
  const HLS_MAX_BUFFER_LENGTH = 30;
  const HLS_MAX_MAX_BUFFER_LENGTH = 60;
  const HLS_LIVE_SYNC_COUNT = 3;
  const HLS_SCRIPT_PATH = '/static/hls.js';
  const HLS_START_LEVEL_AUTO = -1;
  const HLS_START_POSITION_DEFAULT = -1;
  const INDEX_INCREMENT = 1;
  const MATH_MIN_OUTPUT = 0;
  const PERCENTAGE_MULTIPLIER = 0.05;
  const RESUME_MIN_SECS = 15;
  const RESUME_START_THRESHOLD = 0;
  const SAVE_INTERVAL_MS = 10_000;
  const SAVE_MIN_DELTA_SECS = 3;
  const TIME_ZERO = 0;

  let channelLogin = $state('');
  let filename = $state('');
  let playbackError = $state<string | null>(null);
  let isLoading = $state(true);
  let isInitialized = $state(false);

  let playerEl = $state<HTMLVideoElement | null>(null);
  let hlsInstance = $state<HlsInstance | null>(null);
  let progressTimer = $state<number | null>(null);
  let lastSavedPosition = $state(TIME_ZERO);
  let resumeTargetPosition = $state<number | null>(null);
  let resumeSettled = $state(false);

  // Calculate end gap as min(20s, 5% of duration) for proper scaling on short videos
  const getEndGapSecs = (duration: number): number =>
    (!Number.isFinite(duration) || duration <= TIME_ZERO)
      ? DEFAULT_FALLBACK_GAP
      : Math.min(DEFAULT_FALLBACK_GAP, duration * PERCENTAGE_MULTIPLIER);

  const goBack = (): void => navigate('/twitch/recordings');

  const hlsPlaylistUrl = (): string => {
    const params = new URLSearchParams({
      channel_login: channelLogin,
      filename
    });
    return `/api/recordings/hls-playlist?${params.toString()}`;
  };

  const waitForHls = (): Promise<void> =>
    new Promise((resolve) => {
      let attempts = TIME_ZERO;

      const checkHls = (): void => {
        if (typeof globalThis !== 'undefined' && 'Hls' in globalThis) {
          resolve();
          return;
        }
        if (attempts < HLS_ATTEMPTS) {
          attempts += INDEX_INCREMENT;
          globalThis.setTimeout(checkHls, HLS_ATTEMPT_DELAY_MS);
        } else {
          resolve();
        }
      };

      checkHls();
    });

  const checkExistingScript = (path: string): Promise<boolean> =>
    new Promise((resolve) => {
      const existing = document.querySelector<HTMLScriptElement>(`script[src="${path}"]`);
      if (existing) {
        waitForHls().then(() => resolve('Hls' in globalThis));
      } else {
        resolve(false);
      }
    });

  const loadScript = (path: string): Promise<boolean> =>
    new Promise((resolve) => {
      const script = document.createElement('script');
      script.src = path;
      script.async = true;

      script.addEventListener('load', () => {
        waitForHls().then(() => resolve('Hls' in globalThis));
      });
      script.addEventListener('error', () => resolve(false));
      document.head.append(script);
    });

  const ensureHlsLoaded = (path: string): Promise<boolean> =>
    new Promise((resolve) => {
      if (typeof globalThis === 'undefined') {
        resolve(false);
        return;
      }
      if ('Hls' in globalThis) {
        resolve(true);
        return;
      }

      checkExistingScript(path).then((found) => {
        if (found) {
          resolve(true);
          return;
        }
        loadScript(path).then(resolve);
      });
    });

  const applyResumePosition = (): void => {
    if (!playerEl || resumeTargetPosition === null) {
      return;
    }

    const { duration } = playerEl;
    const endGap = getEndGapSecs(duration);
    const maxResume = (Number.isFinite(duration) && duration > TIME_ZERO)
      ? Math.max(MATH_MIN_OUTPUT, duration - endGap)
      : Number.POSITIVE_INFINITY;

    if (resumeTargetPosition > maxResume) {
      resumeTargetPosition = null;
      resumeSettled = true;
      return;
    }

    try {
      playerEl.currentTime = resumeTargetPosition;
      resumeSettled = true;
    } catch {
      // No-op
    }
  };

  const shouldSkipProgress = (force: boolean, currentTime: number): boolean =>
    !force && Math.abs(currentTime - lastSavedPosition) < SAVE_MIN_DELTA_SECS;

  const saveProgress = (
    durationSecs: number | undefined,
    currentTime: number
  ): Promise<void> => {
    const endGap = durationSecs ? getEndGapSecs(durationSecs) : DEFAULT_FALLBACK_GAP;
    const isCompleted = typeof durationSecs === 'number'
      && durationSecs > TIME_ZERO
      && durationSecs - currentTime <= endGap;
    lastSavedPosition = currentTime;

    return saveRecordingWatchProgress({
      channel_login: channelLogin,
      completed: isCompleted,
      duration_secs: durationSecs,
      filename,
      position_secs: currentTime,
    }).then(() => {
      // Keep playback uninterrupted if saving progress fails
    }).catch(() => {
      // Keep playback uninterrupted if saving progress fails
    });
  };

  const pushProgress = (force = false): Promise<void> =>
    new Promise((resolve) => {
      if (!channelLogin || !filename || !playerEl) {
        resolve();
        return;
      }

      const { currentTime, duration: durationVal } = playerEl;
      const durationSecs = (Number.isFinite(durationVal) && durationVal > TIME_ZERO)
        ? durationVal
        : undefined;

      if (!Number.isFinite(currentTime) || currentTime < TIME_ZERO) {
        resolve();
        return;
      }

      if (shouldSkipProgress(force, currentTime)) {
        resolve();
        return;
      }

      saveProgress(durationSecs, currentTime).then(() => resolve());
    });

  const onBeforeUnload = (): void => {
    pushProgress(true).catch(() => {
      // Intentionally empty
    });
  };

  const onVisibilityChange = (): void => {
    if (document.visibilityState === 'hidden') {
      pushProgress(true);
    }
  };

  const stopProgressTracking = (): void => {
    if (typeof globalThis === 'undefined') {
      return;
    }

    if (progressTimer !== null) {
      globalThis.clearInterval(progressTimer);
      progressTimer = null;
    }
    globalThis.removeEventListener('beforeunload', onBeforeUnload);
    document.removeEventListener('visibilitychange', onVisibilityChange);
  };

  const setupProgressTracking = (): void => {
    if (typeof globalThis === 'undefined') {
      return;
    }

    stopProgressTracking();
    progressTimer = globalThis.setInterval(() => {
      pushProgress(false);
    }, SAVE_INTERVAL_MS) as unknown as number;
    globalThis.addEventListener('beforeunload', onBeforeUnload);
    document.addEventListener('visibilitychange', onVisibilityChange);
  };

  const createHlsInstance = (resumeAt: number | null): HlsInstance | null => {
    if (typeof globalThis === 'undefined') {
      return null;
    }
    if (!('Hls' in globalThis)) {
      return null;
    }

    const HlsClass = (globalThis as unknown as { Hls: HlsStatic }).Hls;
    if (!HlsClass.isSupported()) {
      return null;
    }

    const startPosition = typeof resumeAt === 'number' ? resumeAt : HLS_START_POSITION_DEFAULT;

    return new HlsClass({
      abrEwmaFastLive: ABR_FAST_LIVE,
      abrEwmaSlowLive: ABR_SLOW_LIVE,
      autoStartLoad: false,
      capLevelToPlayerSize: true,
      enableWorker: false,
      liveSyncDurationCount: HLS_LIVE_SYNC_COUNT,
      maxBufferLength: HLS_MAX_BUFFER_LENGTH,
      maxMaxBufferLength: HLS_MAX_MAX_BUFFER_LENGTH,
      startLevel: HLS_START_LEVEL_AUTO,
      startPosition,
    });
  };

  const handleManifestParsed = (
    hlsRuntime: { startLoad?: (position: number) => void } | null,
    resumeAt: number | null,
    hideLoading: () => void
  ): void => {
    const startPos = (typeof resumeAt === 'number' && Number.isFinite(resumeAt) && resumeAt > RESUME_START_THRESHOLD)
      ? resumeAt
      : HLS_START_POSITION_DEFAULT;

    if (hlsRuntime && hlsRuntime.startLoad) {
      hlsRuntime.startLoad(startPos);
    }
    hideLoading();
  };

  interface HlsErrorHandlerContext {
    hls: HlsInstance;
    HlsClass: HlsStatic;
    hideLoading: () => void;
    resolve: (value: boolean) => void;
  }

  const setupHlsErrorHandler = (ctx: HlsErrorHandlerContext): void => {
    ctx.hls.on(ctx.HlsClass.Events.ERROR, (_event: unknown, data: unknown) => {
      const errorData = data as { fatal?: boolean };
      if (errorData.fatal) {
        ctx.resolve(false);
      } else {
        ctx.hideLoading();
      }
    });
  };

  interface HlsEventListenersContext {
    hls: HlsInstance;
    HlsClass: HlsStatic;
    resumeAt: number | null;
    hideLoading: () => void;
    resolve: (value: boolean) => void;
  }

  const setupHlsEventListeners = (ctx: HlsEventListenersContext): void => {
    ctx.hls.on(ctx.HlsClass.Events.MANIFEST_PARSED, (_event: unknown, _data: unknown) => {
      const hlsRuntime = hlsInstance as unknown as { startLoad?: (position: number) => void } | null;
      handleManifestParsed(hlsRuntime, ctx.resumeAt, ctx.hideLoading);
      ctx.resolve(true);
    });

    setupHlsErrorHandler({
      HlsClass: ctx.HlsClass,
      hideLoading: ctx.hideLoading,
      hls: ctx.hls,
      resolve: ctx.resolve,
    });
  };

  const setupPlayerEventListeners = (hideLoading: () => void): void => {
    if (!playerEl) {
      return;
    }

    playerEl.addEventListener('loadedmetadata', () => {
      if (resumeTargetPosition !== null && !resumeSettled) {
        applyResumePosition();
      }
      hideLoading();
    }, { once: true });

    playerEl.addEventListener('loadeddata', () => {
      if (resumeTargetPosition !== null && !resumeSettled) {
        applyResumePosition();
      }
      hideLoading();
    }, { once: true });

    globalThis.setTimeout(hideLoading, HLS_LOAD_TIMEOUT_MS);
  };

  const trySetupHls = (
    HlsClass: HlsStatic,
    resumeAt: number | null,
    resolve: (value: boolean) => void
  ): void => {
    if (!HlsClass.isSupported()) {
      resolve(false);
      return;
    }

    hlsInstance = createHlsInstance(resumeAt);

    if (!hlsInstance || !playerEl) {
      resolve(false);
      return;
    }

    const hideLoading = (): void => { isLoading = false; };

    setupHlsEventListeners({
      HlsClass,
      hideLoading,
      hls: hlsInstance,
      resumeAt,
      resolve,
    });
    setupPlayerEventListeners(hideLoading);

    hlsInstance.loadSource(hlsPlaylistUrl());
    hlsInstance.attachMedia(playerEl);
  };

  const waitForHlsAndSetup = (resumeAt: number | null, resolve: (value: boolean) => void): void => {
    let attempts = TIME_ZERO;

    const checkAndSetup = (): void => {
      if (typeof globalThis !== 'undefined' && 'Hls' in globalThis) {
    const HlsClass = (globalThis as unknown as { Hls: HlsStatic }).Hls;
        trySetupHls(HlsClass, resumeAt, resolve);
        return;
      }
      if (attempts < HLS_ATTEMPTS) {
        attempts += INDEX_INCREMENT;
        globalThis.setTimeout(checkAndSetup, HLS_ATTEMPT_DELAY_MS);
      } else {
        resolve(false);
      }
    };

    checkAndSetup();
  };

  const loadHlsPlayer = (): Promise<boolean> =>
    new Promise((resolve) => {
      if (!playerEl) {
        resolve(false);
        return;
      }

      setupProgressTracking();
      const resumeAt: number | null = resumeTargetPosition;

      ensureHlsLoaded(HLS_SCRIPT_PATH).then((hlsLoaded) => {
        if (!hlsLoaded) {
          resolve(false);
          return;
        }
        waitForHlsAndSetup(resumeAt, resolve);
      });
    });

  const checkHlsAvailable = (): Promise<boolean> =>
    fetch(hlsPlaylistUrl(), { method: 'HEAD' })
      .then((response) => response.ok)
      .catch(() => false);

  const loadStoredProgress = (): Promise<void> =>
    new Promise((resolve) => {
      if (!channelLogin || !filename) {
        resolve();
        return;
      }

      getRecordingWatchProgress(channelLogin, filename)
        .then((progress) => {
          if (
            !progress.completed
            && typeof progress.position_secs === 'number'
            && progress.position_secs >= RESUME_MIN_SECS
          ) {
            resumeTargetPosition = progress.position_secs;
            resumeSettled = false;
            lastSavedPosition = progress.position_secs;
          }
          resolve();
        })
        .catch(() => {
          // Keep playback uninterrupted if loading progress fails
          resolve();
        });
    });

  const initializePlayer = (): Promise<void> =>
    new Promise((resolve) => {
      playbackError = null;
      loadStoredProgress().then(() => {
        checkHlsAvailable().then((hlsAvailable) => {
          if (!hlsAvailable) {
            playbackError = 'HLS playlist not available for this recording.';
            isLoading = false;
            resolve();
            return;
          }

          loadHlsPlayer().then((hlsLoaded) => {
            if (!hlsLoaded) {
              playbackError = 'Failed to load HLS player.';
              isLoading = false;
            }
            resolve();
          });
        });
      });
    });

  // Get query params from router
  $effect(() => {
    const { query } = (page as unknown as { query?: Record<string, string> });
    channelLogin = (query && query.channel_login) ? query.channel_login : '';
    filename = (query && query.filename) ? query.filename : '';
  });

  // Initialize player when element is ready and we have params
  $effect(() => {
    if (!playerEl || !channelLogin || !filename || isInitialized) {
      return;
    }

    isInitialized = true;
    initializePlayer();
  });

  onDestroy(() => {
    pushProgress(true);
    stopProgressTracking();
    if (hlsInstance) {
      hlsInstance.destroy();
      hlsInstance = null;
    }
    if (playerEl) {
      playerEl.src = '';
      playerEl.load();
    }
  });
</script>

<section class="ui-page-panel ui-page-panel--wide">
  <header class="ui-page-header">
    <div>
      <p class="ui-page-eyebrow">Recording Playback</p>
      <h1 class="ui-page-title">{channelLogin || 'unknown channel'}</h1>
      {#if filename}
        <p class="ui-page-subtle" title={filename}>{filename}</p>
      {/if}
    </div>
    <button type="button" class="ui-nav-chip" onclick={goBack}>Back to recordings</button>
  </header>

  {#if !channelLogin || !filename}
    <p class="ui-error">Missing recording playback parameters.</p>
  {:else}
    <div class="player-wrapper">
      {#if isLoading}
        <div class="player loading">
          <p class="ui-muted">Loading player...</p>
        </div>
      {/if}
      <video class="player" class:hidden={isLoading} controls preload="auto" bind:this={playerEl}>
        Your browser cannot play this recording format.
      </video>
    </div>
    {#if playbackError}
      <p class="ui-error" role="alert">{playbackError}</p>
    {/if}
  {/if}
</section>

<style>
  /* Player-specific styles - not shared */
  .ui-page-panel--wide {
    display: grid;
    gap: 0.8rem;
  }

  .player-wrapper {
    width: 100%;
    aspect-ratio: 16 / 9;
    min-height: 16rem;
    max-height: min(74vh, 52rem);
    border: 1px solid rgba(180, 198, 236, 0.35);
    background: #000;
    overflow: hidden;
  }

  .player {
    width: 100%;
    height: 100%;
    border-radius: 0;
    border: none;
    background: #000;
    display: block;
  }

  .player.loading {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .player.hidden {
    display: none;
  }

  /* 720p-class landscape TV browsers (e.g., Xbox Edge) */
  @media screen
    and (min-width: 1000px)
    and (max-width: 1400px)
    and (min-height: 600px)
    and (max-height: 800px)
    and (orientation: landscape) {
    .player-wrapper {
      max-height: min(70vh, 600px);
    }
  }
</style>
