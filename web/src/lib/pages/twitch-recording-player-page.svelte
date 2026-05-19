<script lang="ts">
  import { onDestroy } from 'svelte';

  import { getRecordingWatchProgress, saveRecordingWatchProgress } from '$lib/api-client';
  import { navigate, page } from '$lib/router/router.svelte';

  const ABR_FAST_LIVE = 3;
  const ABR_SLOW_LIVE = 9;
  const DEFAULT_FALLBACK_GAP = 20;
  const HLS_ATTEMPTS = 50;
  const HLS_ATTEMPT_DELAY_MS = 100;
  const HLS_LOAD_TIMEOUT_MS = 3000;
  const HLS_MAX_BUFFER_LENGTH = 30;
  const HLS_MAX_MAX_BUFFER_LENGTH = 60;
  const HLS_LIVE_SYNC_COUNT = 3;
  const PERCENTAGE_MULTIPLIER = 0.05;
  const RESUME_MIN_SECS = 15;
  const SAVE_INTERVAL_MS = 10_000;
  const SAVE_MIN_DELTA_SECS = 3;

  let channelLogin = $state('');
  let filename = $state('');
  let playbackError = $state<string | undefined>(undefined);
  let isLoading = $state(true);
  let isInitialized = $state(false);

  let playerEl = $state<HTMLVideoElement | undefined>(undefined);
  let hlsInstance = $state<Hls | undefined>(undefined);
  let progressTimer = $state<number | undefined>(undefined);
  let lastSavedPosition = $state(0);
  let resumeTargetPosition = $state<number | undefined>(undefined);
  let resumeSettled = $state(false);

  // Calculate end gap as min(20s, 5% of duration) for proper scaling on short videos
  const getEndGapSecs = (duration: number): number => {
    if (!Number.isFinite(duration) || duration <= 0) {
      return DEFAULT_FALLBACK_GAP;
    }
    return Math.min(DEFAULT_FALLBACK_GAP, duration * PERCENTAGE_MULTIPLIER);
  };

  // Get query params from router
  $effect(() => {
    const { query } = (page as unknown as { query?: Record<string, string> });
    channelLogin = query?.channel_login ?? '';
    filename = query?.filename ?? '';
  });

  // Initialize player when element is ready and we have params
  $effect(() => {
    if (!playerEl || !channelLogin || !filename || isInitialized) {
      return;
    }

    isInitialized = true;
    initializePlayer();
  });

  const initializePlayer = async (): Promise<void> => {
    playbackError = undefined;
    await loadStoredProgress();

    // HLS is required - check if playlist exists
    const hlsAvailable = await checkHlsAvailable();
    if (!hlsAvailable) {
      playbackError = 'HLS playlist not available for this recording.';
      isLoading = false;
      return;
    }

    const hlsLoaded = await loadHlsPlayer();
    if (!hlsLoaded) {
      playbackError = 'Failed to load HLS player.';
      isLoading = false;
    }
  };

  const goBack = (): void => {
    navigate('/twitch/recordings');
  };

  const hlsPlaylistUrl = (): string => {
    const params = new URLSearchParams({
      channel_login: channelLogin,
      filename
    });
    return `/api/recordings/hls-playlist?${params.toString()}`;
  };

  const checkHlsAvailable = async (): Promise<boolean> => {
    try {
      const response = await fetch(hlsPlaylistUrl(), { method: 'HEAD' });
      return response.ok;
    } catch {
      return false;
    }
  };

  const loadHlsPlayer = async (): Promise<boolean> => {
    if (!playerEl) {
      return false;
    }
    setupProgressTracking();
    const resumeAt: number | undefined = resumeTargetPosition;

  // Wait for hls.js to load (it may still be loading from the script tag)
    let attempts = 0;
    while (typeof globalThis !== 'undefined' && !('Hls' in globalThis) && attempts < HLS_ATTEMPTS) {
      await new Promise((resolve) => {
        globalThis.setTimeout(resolve, HLS_ATTEMPT_DELAY_MS);
      });
      attempts += 1;
    }

    // Check if HLS.js is available
    if (typeof globalThis !== 'undefined' && 'Hls' in globalThis && Hls.isSupported()) {
      const HlsClass = (globalThis as unknown as { Hls: typeof Hls }).Hls;
      hlsInstance = new HlsClass({
        abrEwmaFastLive: ABR_FAST_LIVE,
        abrEwmaSlowLive: ABR_SLOW_LIVE,
        autoStartLoad: false,
        capLevelToPlayerSize: true,
        enableWorker: false,
        liveSyncDurationCount: HLS_LIVE_SYNC_COUNT,
        maxBufferLength: HLS_MAX_BUFFER_LENGTH,
        maxMaxBufferLength: HLS_MAX_MAX_BUFFER_LENGTH,
        startLevel: -1,
        startPosition: typeof resumeAt === 'number' ? resumeAt : -1,
      });

      return new Promise((resolve) => {
        // Hide spinner when video is ready to play
        const hideLoading = () => {
          isLoading = false;
        };

        if (!hlsInstance || !playerEl) {
          resolve(false);
          return;
        }



        // Listen for manifest parsed (HLS ready)
        hlsInstance.on(HlsClass.Events.MANIFEST_PARSED, (_event: unknown, _data: unknown) => {
          const hlsRuntime = hlsInstance as unknown as { startLoad?: (position: number) => void } | null;
          const startPos: number = typeof resumeAt === 'number' && Number.isFinite(resumeAt) && resumeAt > 0 ? resumeAt : -1;
          hlsRuntime?.startLoad?.(startPos);
          hideLoading();
          resolve(true);
        });

        // Handle errors
        hlsInstance.on(HlsClass.Events.ERROR, (_event: unknown, data: unknown) => {
          const errorData = data as { fatal?: boolean };
          if (errorData.fatal) {
            resolve(false);
          } else {
            hideLoading();
          }
        });

        // Resume: seek when metadata is available
        playerEl.addEventListener('loadedmetadata', () => {
          if (resumeTargetPosition !== undefined && !resumeSettled) {
            applyResumePosition();
          }
          hideLoading();
        }, { once: true });

        // Resume: seek when first frame is available
        playerEl.addEventListener('loadeddata', () => {
          if (resumeTargetPosition !== undefined && !resumeSettled) {
            applyResumePosition();
          }
          hideLoading();
        }, { once: true });

        // Fallback: timeout
        setTimeout(() => {
          hideLoading();
        }, HLS_LOAD_TIMEOUT_MS);

        // Start loading
        hlsInstance.loadSource(hlsPlaylistUrl());
        hlsInstance.attachMedia(playerEl);
      });
    }

    // No native HLS fallback - hls.js required for byte-range playlists
    return false;
  }

  onDestroy(() => {
    pushProgress(true);
    stopProgressTracking();
    if (hlsInstance) {
      hlsInstance.destroy();
      hlsInstance = undefined;
    }
    if (playerEl) {
      playerEl.src = '';
      playerEl.load();
    }
  });

  const loadStoredProgress = async (): Promise<void> => {
    if (!channelLogin || !filename) {
      return;
    }
    try {
      const progress = await getRecordingWatchProgress(channelLogin, filename);
      if (
        !progress.completed &&
        typeof progress.position_secs === 'number' &&
        progress.position_secs >= RESUME_MIN_SECS
      ) {
        resumeTargetPosition = progress.position_secs;
        resumeSettled = false;
        lastSavedPosition = progress.position_secs;
      }
    } catch {
      // keep playback uninterrupted if loading progress fails
    }
  };

  const applyResumePosition = (): void => {
    if (!playerEl || resumeTargetPosition === undefined) {
      return;
    }
    const { duration } = playerEl;
    const endGap = getEndGapSecs(duration);
    const maxResume = Number.isFinite(duration) && duration > 0
      ? Math.max(0, duration - endGap)
      : Number.POSITIVE_INFINITY;
    if (resumeTargetPosition > maxResume) {
      resumeTargetPosition = undefined;
      resumeSettled = true;
      return;
    }
    try {
      playerEl.currentTime = resumeTargetPosition;
      resumeSettled = true;
    } catch {
      // no-op
    }
  };

  const pushProgress = async (force = false): Promise<void> => {
    if (!channelLogin || !filename || !playerEl) {
      return;
    }
    const { currentTime, duration: durationVal } = playerEl;
    const durationSecs = Number.isFinite(durationVal) && durationVal > 0 ? durationVal : undefined;
    if (!Number.isFinite(currentTime) || currentTime < 0) {
      return;
    }
    if (!force && Math.abs(currentTime - lastSavedPosition) < SAVE_MIN_DELTA_SECS) {
      return;
    }

    const endGap: number = durationSecs ? getEndGapSecs(durationSecs) : DEFAULT_FALLBACK_GAP;
    const isCompleted =
      typeof durationSecs === 'number' && durationSecs > 0 && durationSecs - currentTime <= endGap;
    lastSavedPosition = currentTime;

    try {
      await saveRecordingWatchProgress({
        channel_login: channelLogin,
        completed: isCompleted,
        duration_secs: durationSecs,
        filename,
        position_secs: currentTime,
      });
    } catch {
      // keep playback uninterrupted if saving progress fails
    }
  };

  const onBeforeUnload = (): void => {
    pushProgress(true);
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
    if (progressTimer !== undefined) {
      globalThis.clearInterval(progressTimer);
      progressTimer = undefined;
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
  }
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
