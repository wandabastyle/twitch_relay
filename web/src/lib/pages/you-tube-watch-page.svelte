<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import { navigate } from '$lib/router/router.svelte';
  import {
    getYouTubeEmbedConfig,
    getYouTubeVideoMeta,
    getYouTubeVideoProgress,
    saveYouTubeVideoProgress,
  } from '$lib/api-client';
  import { Chat, VideoPlayer } from '$lib/components/watch';

  const DEFAULT_END_GAP = 20;
  const DEFAULT_REFERRER_POLICY: 'no-referrer' | 'strict-origin-when-cross-origin' = 'no-referrer';
  const ERROR_NO_ID = 'No video ID provided.';
  const ERROR_LOAD_FAILED = 'Failed to load embed configuration.';
  const FALLBACK_VIDEO_TITLE = 'YouTube video';
  const HOURS_IN_SECONDS = 3600;
  const MINUTES_IN_SECONDS = 60;
  const PAD_LENGTH = 2;
  const PERCENTAGE_MULTIPLIER = 0.05;
  const RESUME_MIN_SECS = 15;
  const SAVE_INTERVAL_MS = 10_000;
  const SAVE_MIN_DELTA_SECS = 3;
  const ZERO = 0;
  const ONE = 1;
  const AUTOPLAY_ON = 1;

  interface Props {
    video_id: string;
  }

  const { video_id }: Props = $props();

  let embedUrl = $state('');
  let referrerPolicy = $state<'no-referrer' | 'strict-origin-when-cross-origin'>(DEFAULT_REFERRER_POLICY);
  let isLoading = $state(false);
  let error = $state<string | null>(null);
  let videoTitle = $state(FALLBACK_VIDEO_TITLE);
  let videoDuration = $state<number | null>(null);
  let playerFrame = $state<HTMLIFrameElement | null>(null);
  let progressTimer = $state<number | null>(null);
  let lastSavedPosition = $state(ZERO);

  // Calculate end gap as min(20s, 5% of duration) for proper scaling on short videos
  const getEndGapSecs = (duration: number): number => {
    if (!Number.isFinite(duration) || duration <= ZERO) {
      return DEFAULT_END_GAP;
    }
    return Math.min(DEFAULT_END_GAP, duration * PERCENTAGE_MULTIPLIER);
  };

  const buildEmbedUrl = (
    id: string,
    defaults: { autoplay: number; quality: string; quality_dash: string },
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

    // Use backend proxy endpoint to avoid Basic auth popup
    return `/api/youtube/embed/${encodeURIComponent(id)}?${params.toString()}`;
  }

  const getEmbeddedVideoElement = (): HTMLVideoElement | null => {
    if (playerFrame === null) {
      return null;
    }
    try {
      const contentWindow = playerFrame.contentWindow;
      if (contentWindow === null) {
        return null;
      }
      const doc = contentWindow.document;
      if (doc === null) {
        return null;
      }
      return doc.querySelector('video');
    } catch {
      return null;
    }
  };

  interface SkipProgressContext {
    force: boolean;
    currentTime: number;
    lastSaved: number;
    minDelta: number;
  }

  const shouldSkipProgressSave = (ctx: SkipProgressContext): boolean =>
    !ctx.force && Math.abs(ctx.currentTime - ctx.lastSaved) < ctx.minDelta;

  const calculateEndGap = (duration: number | null): number => {
    if (typeof duration === 'number' && duration > ZERO) {
      return getEndGapSecs(duration);
    }
    return DEFAULT_END_GAP;
  };

  const checkVideoCompleted = (
    duration: number | null,
    currentTime: number,
    endGap: number,
  ): boolean => {
    if (typeof duration !== 'number' || duration <= ZERO) {
      return false;
    }
    return duration - currentTime <= endGap;
  };

  const getDurationValue = (duration: number | null): number | null => {
    if (typeof duration === 'number') {
      return duration;
    }
    return null;
  };

  const pushProgress = (force = false): Promise<void> => {
    return new Promise((resolve) => {
      if (video_id === '') {
        resolve();
        return;
      }
      const video = getEmbeddedVideoElement();
      if (video === null) {
        resolve();
        return;
      }
      const currentTime = video.currentTime;
      const videoDur = video.duration;
      let duration: number | null;
      if (Number.isFinite(videoDur) && videoDur > ZERO) {
        duration = videoDur;
      } else {
        duration = videoDuration;
      }
      if (!Number.isFinite(currentTime) || currentTime < ZERO) {
        resolve();
        return;
      }
      if (shouldSkipProgressSave({
        currentTime,
        force,
        lastSaved: lastSavedPosition,
        minDelta: SAVE_MIN_DELTA_SECS,
      })) {
        resolve();
        return;
      }

      const endGap = calculateEndGap(duration);
      const isCompleted = checkVideoCompleted(duration, currentTime, endGap);
      lastSavedPosition = currentTime;

      const durationValue = getDurationValue(duration);
      saveYouTubeVideoProgress(video_id, {
        completed: isCompleted,
        duration_secs: durationValue,
        position_secs: currentTime,
      })
        .then(() => {
          resolve();
        })
        .catch(() => {
          // Intentionally empty: keep playback uninterrupted if saving progress fails
          resolve();
        });
    });
  }

  // Event handlers (defined after pushProgress since they use it)
  const onBeforeUnload = (): void => {
    pushProgress(true).catch(() => {
      // Intentionally empty: keep playback uninterrupted if saving progress fails
    });
  };

  const onVisibilityChange = (): void => {
    if (document.visibilityState === 'hidden') {
      pushProgress(true).catch(() => {
        // Intentionally empty: keep playback uninterrupted if saving progress fails
      });
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

  const startProgressTracking = (): void => {
    if (typeof globalThis === 'undefined') {
      return;
    }
    stopProgressTracking();
    progressTimer = globalThis.setInterval(() => {
      pushProgress(false).catch(() => {
        // Intentionally empty: keep playback uninterrupted if saving progress fails
      });
    }, SAVE_INTERVAL_MS) as unknown as number;
    globalThis.addEventListener('beforeunload', onBeforeUnload);
    document.addEventListener('visibilitychange', onVisibilityChange);
  };

  const goBack = (): void => {
    // Check if we have a stored return URL in history state
    // (set by list pages when navigating to watch)
    const state = history.state;
    const returnUrl =
      state !== null && typeof state === 'object' && 'youtubeReturnUrl' in state
        ? state.youtubeReturnUrl
        : null;
    if (typeof globalThis !== 'undefined' && returnUrl !== null) {
      // Use history.back() for natural scroll position restoration
      globalThis.history.back();
      return;
    }

    // Fallback
    if (globalThis.history.length > ONE) {
      globalThis.history.back();
    } else {
      navigate('/youtube');
    }
  };

  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / HOURS_IN_SECONDS);
    const minutes = Math.floor((seconds % HOURS_IN_SECONDS) / MINUTES_IN_SECONDS);
    const secs = seconds % MINUTES_IN_SECONDS;

    if (hours > ZERO) {
      return `${hours}:${minutes.toString().padStart(PAD_LENGTH, '0')}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
    }
    return `${minutes}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
  }

  const determineResumePosition = (
    progress: { completed: boolean; position_secs: number | null },
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

  const updateReferrerPolicy = (referrerPolicyValue: string | undefined): void => {
    if (referrerPolicyValue === 'strict-origin-when-cross-origin') {
      referrerPolicy = 'strict-origin-when-cross-origin';
    } else {
      referrerPolicy = DEFAULT_REFERRER_POLICY;
    }
  };

  const handleEmbedConfig = (
    config: { defaults: { autoplay: number; quality: string; quality_dash: string }; referrer_policy?: string },
    meta: { duration: number; title: string },
    progress: { completed: boolean; position_secs: number | null },
  ): void => {
    const endGap = getEndGapSecs(meta.duration);
    const resumeAt = determineResumePosition(progress, meta.duration, endGap);
    if (resumeAt !== null) {
      lastSavedPosition = resumeAt;
    }
    embedUrl = buildEmbedUrl(video_id, config.defaults, resumeAt);
    videoTitle = meta.title;
    videoDuration = meta.duration;
    updateReferrerPolicy(config.referrer_policy);
    startProgressTracking();
  };

  const handleLoadError = (error_: unknown): void => {
    let errorMessage: string;
    if (error_ instanceof Error) {
      errorMessage = error_.message;
    } else {
      errorMessage = ERROR_LOAD_FAILED;
    }
    error = errorMessage;
  };

  onMount(() => {
    if (video_id === '') {
      error = ERROR_NO_ID;
      return;
    }

    isLoading = true;
    error = null;

    Promise.all([
      getYouTubeEmbedConfig(),
      getYouTubeVideoMeta(video_id),
      getYouTubeVideoProgress(video_id),
    ])
      .then(([config, meta, progress]) => {
        handleEmbedConfig(config, meta, progress);
      })
      .catch((error_) => {
        handleLoadError(error_);
      })
      .finally(() => {
        isLoading = false;
      });
  });

  onDestroy(() => {
    stopProgressTracking();
  });
</script>

<section class="ui-page-panel ui-page-panel--wide">
  <header class="player-header">
    <div>
      <button type="button" class="ui-nav-chip" onclick={goBack}>Back to videos</button>
      <h1>{videoTitle}</h1>
      {#if videoDuration !== null}
        <p class="subtle">Duration: {formatDuration(videoDuration)}</p>
      {/if}
    </div>
  </header>

  {#if error !== null}
    <div class="player-wrapper">
      <div class="player error-box">
        <p class="ui-error" role="alert">{error}</p>
      </div>
    </div>
  {:else if isLoading}
    <div class="player-wrapper">
      <div class="player loading-box">
        <p class="ui-muted">Loading video...</p>
      </div>
    </div>
  {:else if video_id !== '' && embedUrl !== ''}
    <div class="player-wrapper">
      <iframe
        bind:this={playerFrame}
        class="player"
        src={embedUrl}
        title="Invidious video player"
        allow="autoplay; fullscreen; encrypted-media; picture-in-picture"
        allowfullscreen
        loading="eager"
        referrerpolicy={referrerPolicy}
      ></iframe>
    </div>
  {:else}
    <div class="player-wrapper">
      <div class="player error-box">
        <p class="ui-error" role="alert">Unable to initialize player.</p>
      </div>
    </div>
  {/if}
</section>

<style>
  .player-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .player-header .ui-nav-chip {
    margin-bottom: 0.5rem;
  }

  /* .nav-chip-btn styles now provided by app.css via .ui-nav-chip */

  h1 {
    margin: 0.2rem 0 0;
    font-size: clamp(1.2rem, 3vw, 1.8rem);
    line-height: 1.3;
  }

  .subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.84rem;
    overflow-wrap: anywhere;
  }

  .player-wrapper {
    width: 100%;
    aspect-ratio: 16 / 9;
    min-height: 16rem;
    max-height: min(74vh, 52rem);
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: #000;
    overflow: hidden;
  }

  .player {
    width: 100%;
    height: 100%;
    border: none;
    display: block;
  }

  .error-box {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: #000;
  }

  .loading-box {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: color-mix(in srgb, var(--bg-soft) 50%, #000);
  }

  /* 720p-class landscape TV browsers (e.g., Xbox Edge) */
  @media screen
    and (min-width: 100px)
    and (max-width: 1400px)
    and (min-height: 600px)
    and (max-height: 800px)
    and (orientation: landscape) {
    .player-wrapper {
      max-height: min(70vh, 600px);
    }
  }
</style>
