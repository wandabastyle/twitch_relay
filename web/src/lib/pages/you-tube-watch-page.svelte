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

  interface Props {
    video_id: string;
  }

  const { video_id }: Props = $props();

  let embedUrl = $state('');
  let referrerPolicy = $state<'no-referrer' | 'strict-origin-when-cross-origin'>(DEFAULT_REFERRER_POLICY);
  let isLoading = $state(false);
  let error = $state<string | undefined>(undefined);
  let videoTitle = $state(FALLBACK_VIDEO_TITLE);
  let videoDuration = $state<number | undefined>(undefined);
  let playerFrame = $state<HTMLIFrameElement | undefined>(undefined);
  let progressTimer = $state<number | undefined>(undefined);
  let lastSavedPosition = $state(0);

  // Calculate end gap as min(20s, 5% of duration) for proper scaling on short videos
  const getEndGapSecs = (duration: number): number => {
    if (!Number.isFinite(duration) || duration <= 0) {
      return DEFAULT_END_GAP;
    }
    return Math.min(DEFAULT_END_GAP, duration * PERCENTAGE_MULTIPLIER);
  };

  const buildEmbedUrl = (
    id: string,
    defaults: { autoplay: number; quality: string; quality_dash: string },
    resumeAtSecs?: number,
  ): string => {
    const params = new URLSearchParams({
      autoplay: String(defaults.autoplay),
      quality: defaults.quality,
      quality_dash: defaults.quality_dash,
    });
    if (resumeAtSecs && resumeAtSecs >= RESUME_MIN_SECS) {
      const resumeSeconds = String(Math.floor(resumeAtSecs));
      params.set('start', resumeSeconds);
      params.set('t', `${resumeSeconds}s`);
    }

    // Use backend proxy endpoint to avoid Basic auth popup
    return `/api/youtube/embed/${encodeURIComponent(id)}?${params.toString()}`;
  }

  onMount(async () => {
    if (!video_id) {
      error = ERROR_NO_ID;
      return;
    }

    isLoading = true;
    error = undefined;

    try {
      const [config, meta, progress] = await Promise.all([
        getYouTubeEmbedConfig(),
        getYouTubeVideoMeta(video_id),
        getYouTubeVideoProgress(video_id),
      ]);
      let resumeAt: number | undefined = undefined;
      const endGap = getEndGapSecs(meta.duration);
      if (
        !progress.completed &&
        typeof progress.position_secs === 'number' &&
        progress.position_secs >= RESUME_MIN_SECS &&
        progress.position_secs <= Math.max(0, meta.duration - endGap)
      ) {
        resumeAt = progress.position_secs;
        lastSavedPosition = progress.position_secs;
      }
      embedUrl = buildEmbedUrl(video_id, config.defaults, resumeAt);
      videoTitle = meta.title;
      videoDuration = meta.duration;
      referrerPolicy = config.referrer_policy === 'strict-origin-when-cross-origin'
        ? 'strict-origin-when-cross-origin'
        : DEFAULT_REFERRER_POLICY;
      startProgressTracking();
    } catch (error_) {
      const errorMessage = error_ instanceof Error ? error_.message : ERROR_LOAD_FAILED;
      error = errorMessage;
    } finally {
      isLoading = false;
    }
  });

  onDestroy(() => {
    stopProgressTracking();
  });

  const getEmbeddedVideoElement = (): HTMLVideoElement | null => {
    if (!playerFrame) {
      return null;
    }
    try {
      const doc = playerFrame.contentWindow?.document;
      if (!doc) {
        return null;
      }
      return doc.querySelector('video');
    } catch {
      return null;
    }
  };

  const pushProgress = async (force = false): Promise<void> => {
    if (!video_id) {
      return;
    }
    const video = getEmbeddedVideoElement();
    if (!video) {
      return;
    }
    const currentTime = video.currentTime;
    const duration =
      Number.isFinite(video.duration) && video.duration > 0 ? video.duration : videoDuration;
    if (!Number.isFinite(currentTime) || currentTime < 0) {
      return;
    }
    if (!force && Math.abs(currentTime - lastSavedPosition) < SAVE_MIN_DELTA_SECS) {
      return;
    }

    const endGap = typeof duration === 'number' && duration > 0 ? getEndGapSecs(duration) : DEFAULT_END_GAP;
    const isCompleted =
      typeof duration === 'number' && duration > 0 && duration - currentTime <= endGap;
    lastSavedPosition = currentTime;

    try {
      await saveYouTubeVideoProgress(video_id, {
        completed: isCompleted,
        duration_secs: duration ?? undefined,
        position_secs: currentTime,
      });
    } catch {
      // keep playback uninterrupted if saving progress fails
    }
  }

  const stopProgressTracking = (): void => {
    if (typeof globalThis === 'undefined') {
      return;
    }
    if (progressTimer !== undefined) {
      globalThis.clearInterval(progressTimer);
      progressTimer = undefined;
    }
    globalThis.removeEventListener('beforeunload', onBeforeUnload);
    document.removeEventListener('visibilitychange', onVisibilityChange);
  };

  const onBeforeUnload = (): void => {
    pushProgress(true).catch(() => {
      // keep playback uninterrupted if saving progress fails
    });
  };

  const onVisibilityChange = (): void => {
    if (document.visibilityState === 'hidden') {
      pushProgress(true).catch(() => {
        // keep playback uninterrupted if saving progress fails
      });
    }
  };

  const startProgressTracking = (): void => {
    if (typeof globalThis === 'undefined') {
      return;
    }
    stopProgressTracking();
    progressTimer = globalThis.setInterval(() => {
      pushProgress(false).catch(() => {
        // keep playback uninterrupted if saving progress fails
      });
    }, SAVE_INTERVAL_MS) as unknown as number;
    globalThis.addEventListener('beforeunload', onBeforeUnload);
    document.addEventListener('visibilitychange', onVisibilityChange);
  };

  const goBack = (): void => {
    // Check if we have a stored return URL in history state
    // (set by list pages when navigating to watch)
    const returnUrl = history.state?.youtubeReturnUrl;
    if (typeof globalThis !== 'undefined' && returnUrl) {
      // Use history.back() for natural scroll position restoration
      globalThis.history.back();
      return;
    }

    // Fallback
    if (globalThis.history.length > 1) {
      globalThis.history.back();
    } else {
      navigate('/youtube');
    }
  };

  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / HOURS_IN_SECONDS);
    const minutes = Math.floor((seconds % HOURS_IN_SECONDS) / MINUTES_IN_SECONDS);
    const secs = seconds % MINUTES_IN_SECONDS;

    if (hours > 0) {
      return `${hours}:${minutes.toString().padStart(PAD_LENGTH, '0')}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
    }
    return `${minutes}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
  }
</script>

<section class="ui-page-panel ui-page-panel--wide">
  <header class="player-header">
    <div>
      <button type="button" class="ui-nav-chip" onclick={goBack}>Back to videos</button>
      <h1>{videoTitle}</h1>
      {#if videoDuration !== undefined}
        <p class="subtle">Duration: {formatDuration(videoDuration)}</p>
      {/if}
    </div>
  </header>

  {#if error}
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
  {:else if video_id && embedUrl}
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
