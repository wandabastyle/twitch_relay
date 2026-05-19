<script lang="ts">
  import { onDestroy } from 'svelte';
  import { navigate, page } from '$lib/router/router.svelte';
  import { getRecordingWatchProgress, saveRecordingWatchProgress } from '$lib/api-client';

  let channelLogin = $state('');
  let filename = $state('');
  let playbackError = $state<string | null>(null);
  let isLoading = $state(true);
  let isInitialized = $state(false);

  let playerEl = $state<HTMLVideoElement | null>(null);
  let hlsInstance = $state<Hls | null>(null);
  let progressTimer = $state<number | null>(null);
  let lastSavedPosition = $state(0);
  let resumeTargetPosition = $state<number | null>(null);
  let resumeSettled = $state(false);

  const RESUME_MIN_SECS = 15;
  const SAVE_INTERVAL_MS = 10_000;
  const SAVE_MIN_DELTA_SECS = 3;

  // Calculate end gap as min(20s, 5% of duration) for proper scaling on short videos
  function getEndGapSecs(duration: number): number {
    if (!Number.isFinite(duration) || duration <= 0) return 20;
    return Math.min(20, duration * 0.05);
  }

  // Get query params from router
  $effect(() => {
    channelLogin = page.query.channel_login ?? '';
    filename = page.query.filename ?? '';
  });

  // Initialize player when element is ready and we have params
  $effect(() => {
    if (!playerEl || !channelLogin || !filename || isInitialized) return;

    isInitialized = true;
    void initializePlayer();
  });

  async function initializePlayer(): Promise<void> {
    playbackError = null;
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
  }

  function goBack(): void {
    navigate('/twitch/recordings');
  }

  function hlsPlaylistUrl(): string {
    const params = new URLSearchParams({
      channel_login: channelLogin,
      filename
    });
    return `/api/recordings/hls-playlist?${params.toString()}`;
  }

  async function checkHlsAvailable(): Promise<boolean> {
    try {
      const response = await fetch(hlsPlaylistUrl(), { method: 'HEAD' });
      return response.ok;
    } catch {
      return false;
    }
  }

  async function loadHlsPlayer(): Promise<boolean> {
    if (!playerEl) return false;
    setupProgressTracking();
    const resumeAt = resumeTargetPosition;

    // Wait for hls.js to load (it may still be loading from the script tag)
    let attempts = 0;
    while (typeof window !== 'undefined' && !('Hls' in window) && attempts < 50) {
      await new Promise(resolve => setTimeout(resolve, 100));
      attempts++;
    }

    // Check if HLS.js is available
    if (typeof window !== 'undefined' && 'Hls' in window && Hls.isSupported()) {
      const HlsClass = (window as unknown as { Hls: typeof Hls }).Hls;
      hlsInstance = new HlsClass({
        maxBufferLength: 30,          // Max buffer ahead in seconds
        maxMaxBufferLength: 60,       // Absolute max buffer
        liveSyncDurationCount: 3,       // For VOD, this affects start position
        enableWorker: false,            // Disable worker for better debugging
        // Add capLevelToPlayerSize to reduce quality initially
        capLevelToPlayerSize: true,
        // Reduce initial load
        startLevel: -1,                 // Auto start level
        startPosition: typeof resumeAt === 'number' ? resumeAt : -1,
        autoStartLoad: false,
        // More conservative loading
        abrEwmaFastLive: 3.0,
        abrEwmaSlowLive: 9.0,
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
          const startPos = typeof resumeAt === 'number' && Number.isFinite(resumeAt) && resumeAt > 0
            ? resumeAt
            : -1;
          hlsRuntime?.startLoad?.(startPos);
          hideLoading();
          resolve(true);
        });

        // Handle errors
        hlsInstance.on(HlsClass.Events.ERROR, (_event: unknown, data: unknown) => {
          console.error('[HLS] Error:', data);
          const errorData = data as { fatal?: boolean };
          if (errorData.fatal) {
            resolve(false);
          } else {
            hideLoading();
          }
        });

        // Resume: seek when metadata is available
        playerEl.addEventListener('loadedmetadata', () => {
          if (resumeTargetPosition !== null && !resumeSettled) {
            applyResumePosition();
          }
          hideLoading();
        }, { once: true });

        // Resume: seek when first frame is available
        playerEl.addEventListener('loadeddata', () => {
          if (resumeTargetPosition !== null && !resumeSettled) {
            applyResumePosition();
          }
          hideLoading();
        }, { once: true });

        // Fallback: timeout
        setTimeout(() => {
          hideLoading();
        }, 3000);

        // Start loading
        hlsInstance.loadSource(hlsPlaylistUrl());
        hlsInstance.attachMedia(playerEl);
      });
    }

    // No native HLS fallback - hls.js required for byte-range playlists
    return false;
  }

  onDestroy(() => {
    void pushProgress(true);
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

  async function loadStoredProgress(): Promise<void> {
    if (!channelLogin || !filename) return;
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
  }

  function applyResumePosition(): void {
    if (!playerEl || resumeTargetPosition === null) return;
    const duration = playerEl.duration;
    const endGap = getEndGapSecs(duration);
    const maxResume =
      Number.isFinite(duration) && duration > 0
        ? Math.max(0, duration - endGap)
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
      // no-op
    }
  }

  async function pushProgress(force = false): Promise<void> {
    if (!channelLogin || !filename || !playerEl) return;
    const currentTime = playerEl.currentTime;
    const durationSecs = Number.isFinite(playerEl.duration) && playerEl.duration > 0 ? playerEl.duration : undefined;
    if (!Number.isFinite(currentTime) || currentTime < 0) return;
    if (!force && Math.abs(currentTime - lastSavedPosition) < SAVE_MIN_DELTA_SECS) return;

    const endGap = durationSecs ? getEndGapSecs(durationSecs) : 20;
    const isCompleted =
      typeof durationSecs === 'number' && durationSecs > 0 && durationSecs - currentTime <= endGap;
    lastSavedPosition = currentTime;

    try {
      await saveRecordingWatchProgress({
        channel_login: channelLogin,
        filename,
        position_secs: currentTime,
        duration_secs: durationSecs,
        completed: isCompleted,
      });
    } catch {
      // keep playback uninterrupted if saving progress fails
    }
  }

  function onBeforeUnload(): void {
    void pushProgress(true);
  }

  function onVisibilityChange(): void {
    if (document.visibilityState === 'hidden') {
      void pushProgress(true);
    }
  }

  function stopProgressTracking(): void {
    if (typeof window === 'undefined') return;
    if (progressTimer !== null) {
      window.clearInterval(progressTimer);
      progressTimer = null;
    }
    window.removeEventListener('beforeunload', onBeforeUnload);
    document.removeEventListener('visibilitychange', onVisibilityChange);
  }

  function setupProgressTracking(): void {
    if (typeof window === 'undefined') return;
    stopProgressTracking();
    progressTimer = window.setInterval(() => {
      void pushProgress(false);
    }, SAVE_INTERVAL_MS);
    window.addEventListener('beforeunload', onBeforeUnload);
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
