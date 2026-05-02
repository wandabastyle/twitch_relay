<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  type HlsCtor = {
    isSupported: () => boolean;
    new (config?: Record<string, unknown>): {
      attachMedia: (video: HTMLVideoElement) => void;
      loadSource: (url: string) => void;
      startLoad: () => void;
      recoverMediaError: () => void;
      on: (event: string, handler: (...args: unknown[]) => void) => void;
      destroy: () => void;
    };
    Events: {
      MEDIA_ATTACHED: string;
      ERROR: string;
    };
    ErrorTypes: {
      NETWORK_ERROR: string;
      MEDIA_ERROR: string;
    };
  };

  let channelLogin = $state('');
  let filename = $state('');
  let playbackError = $state<string | null>(null);

  let playerEl = $state<HTMLVideoElement | null>(null);
  let hlsInstance: {
    attachMedia: (video: HTMLVideoElement) => void;
    loadSource: (url: string) => void;
    startLoad: () => void;
    recoverMediaError: () => void;
    on: (event: string, handler: (...args: unknown[]) => void) => void;
    destroy: () => void;
  } | null = null;

  let networkRecoveryAttempts = 0;
  let mediaRecoveryAttempts = 0;

  function getHlsCtor(): HlsCtor | null {
    if (typeof window === 'undefined') {
      return null;
    }

    const candidate = (window as Window & { Hls?: HlsCtor }).Hls;
    return candidate ?? null;
  }

  function playlistUrl(): string {
    const params = new URLSearchParams({
      channel_login: channelLogin,
      filename
    });
    return `/api/recordings/playlist.m3u8?${params.toString()}`;
  }

  if (typeof window !== 'undefined') {
    const params = new URLSearchParams(window.location.search);
    channelLogin = params.get('channel_login') || '';
    filename = params.get('filename') || '';
  }

  function goBack(): void {
    window.location.assign('/?view=recordings');
  }

  onMount(() => {
    if (!playerEl || !channelLogin || !filename) {
      return;
    }

    const sourceUrl = playlistUrl();
    playbackError = null;

    if (playerEl.canPlayType('application/vnd.apple.mpegurl')) {
      playerEl.src = sourceUrl;
      return;
    }

    const Hls = getHlsCtor();
    if (Hls?.isSupported()) {
      hlsInstance = new Hls({
        enableWorker: true
      });
      hlsInstance.attachMedia(playerEl);
      hlsInstance.on(Hls.Events.MEDIA_ATTACHED, () => {
        hlsInstance?.loadSource(sourceUrl);
      });
      hlsInstance.on(Hls.Events.ERROR, (_event, data) => {
        const details = data as { fatal?: boolean; type?: string };
        if (details.fatal) {
          if (details.type === Hls.ErrorTypes.NETWORK_ERROR && networkRecoveryAttempts < 2) {
            networkRecoveryAttempts += 1;
            hlsInstance?.startLoad();
            return;
          }

          if (details.type === Hls.ErrorTypes.MEDIA_ERROR && mediaRecoveryAttempts < 1) {
            mediaRecoveryAttempts += 1;
            hlsInstance?.recoverMediaError();
            return;
          }

          hlsInstance?.destroy();
          hlsInstance = null;
          playbackError = 'Playback failed for this recording.';
        }
      });
      return;
    }

    playbackError = 'This browser does not support HLS playback.';
  });

  onDestroy(() => {
    hlsInstance?.destroy();
    hlsInstance = null;
  });
</script>

<svelte:head>
  <title>Recording Playback - Twitch Relay</title>
  <script src="/hls.js"></script>
</svelte:head>

<main class="shell">
  <section class="panel">
    <header class="player-header">
      <div>
        <p class="eyebrow">Recording Playback</p>
        <h1>{channelLogin || 'unknown channel'}</h1>
        {#if filename}
          <p class="subtle" title={filename}>{filename}</p>
        {/if}
      </div>
      <button type="button" class="nav-chip-btn" onclick={goBack}>Back to recordings</button>
    </header>

    {#if !channelLogin || !filename}
      <p class="error">Missing recording playback parameters.</p>
    {:else}
      <video class="player" controls preload="metadata" bind:this={playerEl}>
        Your browser cannot play this recording format.
      </video>
      <p class="hint">Using packaged HLS playback for browser compatibility.</p>
      {#if playbackError}
        <p class="error" role="alert">{playbackError}</p>
      {/if}
    {/if}
  </section>
</main>

<style>
  :global(body) {
    --bg: #1e2030;
    --fg: #c8d3f5;
    --muted: #a9b8e8;
    --accent: #82aaff;
    --danger: #ff757f;
    margin: 0;
    min-height: 100vh;
    background: radial-gradient(circle at 20% -10%, #3b4261 0%, #222436 45%, #1e2030 100%);
    color: var(--fg);
    font-family: 'Space Grotesk', 'IBM Plex Sans', 'Noto Sans', sans-serif;
  }

  .shell {
    min-height: 100dvh;
    box-sizing: border-box;
    display: grid;
    justify-items: center;
    align-content: start;
    padding: 1rem;
  }

  .panel {
    width: min(74rem, 96vw);
    background: linear-gradient(160deg, rgba(47, 51, 77, 0.95), rgba(34, 36, 54, 0.95));
    border: 1px solid rgba(68, 74, 115, 0.65);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
    display: grid;
    gap: 0.8rem;
  }

  .player-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  h1 {
    margin: 0.18rem 0 0;
    font-size: clamp(1.35rem, 3vw, 1.85rem);
    line-height: 1.1;
  }

  .subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.84rem;
    overflow-wrap: anywhere;
  }

  .player {
    width: 100%;
    border-radius: 0;
    border: 1px solid rgba(180, 198, 236, 0.35);
    background: #000;
    min-height: 16rem;
    object-fit: contain;
  }

  .nav-chip-btn {
    background: transparent;
    border: 1px solid rgba(162, 182, 217, 0.45);
    border-radius: 0.6rem;
    color: var(--fg);
    padding: 0.4rem 0.8rem;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    line-height: 1;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 2rem;
  }

  .nav-chip-btn:hover {
    border-color: rgba(190, 206, 234, 0.72);
    background: rgba(17, 26, 41, 0.72);
  }

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.83rem;
  }

  .error {
    margin: 0;
    border-radius: 0.6rem;
    padding: 0.65rem 0.72rem;
    background: rgba(194, 67, 89, 0.18);
    border: 1px solid rgba(246, 135, 154, 0.45);
    color: color-mix(in srgb, var(--danger) 72%, white);
  }

  @media (min-width: 1100px) {
    .shell {
      padding: 0.75rem 1rem;
    }

    .player {
      height: min(74vh, 52rem);
    }
  }
</style>
