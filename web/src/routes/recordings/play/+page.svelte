<script lang="ts">
  import { onDestroy } from 'svelte';
  import AppVersion from '$lib/components/AppVersion.svelte';

  let channelLogin = $state('');
  let filename = $state('');
  let playbackError = $state<string | null>(null);
  let isLoading = $state(true);
  let isInitialized = $state(false);

  let playerEl = $state<HTMLVideoElement | null>(null);
  let hlsInstance = $state<Hls | null>(null);

  // Parse URL params on client side
  $effect(() => {
    if (typeof window !== 'undefined' && !channelLogin) {
      const params = new URLSearchParams(window.location.search);
      channelLogin = params.get('channel_login') || '';
      filename = params.get('filename') || '';
    }
  });

  // Initialize player when element is ready and we have params
  $effect(() => {
    if (!playerEl || !channelLogin || !filename || isInitialized) return;

    isInitialized = true;
    void initializePlayer();
  });

  async function initializePlayer(): Promise<void> {
    playbackError = null;

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
    window.location.assign('/?view=recordings');
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
        // More conservative loading
        abrEwmaFastLive: 3.0,
        abrEwmaSlowLive: 9.0,
      });

      return new Promise((resolve) => {
        // Hide spinner when video is ready to play
        const hideLoading = () => {
          console.log('[Player] Hiding loading spinner');
          isLoading = false;
        };

        if (!hlsInstance || !playerEl) {
          resolve(false);
          return;
        }

        // Listen for manifest parsed (HLS ready)
        hlsInstance.on(HlsClass.Events.MANIFEST_PARSED, () => {
          console.log('[HLS] Manifest parsed');
          hideLoading();
          resolve(true);
        });

        // Handle errors
        hlsInstance.on(HlsClass.Events.ERROR, (_event: unknown, data: unknown) => {
          console.error('[HLS] Error:', data);
          const errorData = data as { fatal?: boolean };
          if (errorData.fatal) {
            console.log('[HLS] Fatal error');
            resolve(false);
          } else {
            console.log('[HLS] Non-fatal error, hiding spinner');
            // Hide spinner on non-fatal errors too - video might still play
            hideLoading();
          }
        });

        // Fallback: hide spinner when video element fires canplay event
        playerEl.addEventListener('canplay', () => {
          console.log('[Video] canplay event');
          hideLoading();
          resolve(true);
        }, { once: true });

        // Fallback: hide on loadedmetadata
        playerEl.addEventListener('loadedmetadata', () => {
          console.log('[Video] loadedmetadata event');
          hideLoading();
        }, { once: true });

        // Fallback: timeout
        setTimeout(() => {
          console.log('[Player] Timeout reached, hiding spinner');
          hideLoading();
          // Don't resolve false - video might still be playing
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

<svelte:head>
  <title>Recording Playback - Twitch Relay</title>
  <script src="/hls.js"></script>
</svelte:head>

<main class="ui-page-shell">
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
  <AppVersion />
</main>

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

  @media (min-width: 1100px) {
    .ui-page-shell {
      padding: 0.75rem 1rem;
    }
  }
</style>