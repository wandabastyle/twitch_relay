<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { resolveYouTubeVideo } from '$lib/api';
  import type { VideoStream } from '$lib/api';

  let player = $state<HTMLVideoElement | null>(null);
  let hls = $state<any | null>(null);
  let stream = $state<VideoStream | null>(null);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let isPlaying = $state(false);

  onMount(async () => {
    const videoId = $page.params.video_id;
    if (!videoId) {
      error = 'No video ID provided';
      isLoading = false;
      return;
    }

    try {
      const resolved = await resolveYouTubeVideo({ video_id: videoId });
      stream = resolved;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load video';
    } finally {
      isLoading = false;
    }
  });

  onDestroy(() => {
    if (hls) {
      hls.destroy();
    }
  });

  function initializePlayer(node: HTMLVideoElement) {
    player = node;

    if (!stream) return;

    if (stream.is_hls) {
      // Try to use hls.js if available
      if (typeof window !== 'undefined' && (window as any).Hls) {
        const Hls = (window as any).Hls;
        if (Hls.isSupported()) {
          hls = new Hls({
            maxBufferLength: 30,
            maxMaxBufferLength: 60,
          });
          hls.loadSource(stream.stream_url);
          hls.attachMedia(node);
          hls.on(Hls.Events.MANIFEST_PARSED, () => {
            node.play().catch(() => {
              // Autoplay prevented, user needs to interact
            });
          });
          hls.on(Hls.Events.ERROR, (_: any, data: any) => {
            if (data.fatal) {
              error = 'Failed to load video stream';
            }
          });
        } else if (node.canPlayType('application/vnd.apple.mpegurl')) {
          // Native HLS support
          node.src = stream.stream_url;
        } else {
          error = 'HLS playback not supported in this browser';
        }
      } else if (node.canPlayType('application/vnd.apple.mpegurl')) {
        // Native HLS support
        node.src = stream.stream_url;
      } else {
        error = 'HLS playback not supported in this browser';
      }
    } else {
      // Direct MP4 playback
      node.src = stream.stream_url;
    }

    node.addEventListener('play', () => {
      isPlaying = true;
    });

    node.addEventListener('pause', () => {
      isPlaying = false;
    });
  }

  function goBack() {
    if (stream) {
      // Go back to channel page
      window.history.back();
    } else {
      goto('/');
    }
  }

  function formatDuration(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;

    if (hours > 0) {
      return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  }
</script>

<svelte:head>
  <title>{stream?.title ?? 'Loading...'} - YouTube Relay</title>
  <!-- Load hls.js from CDN for HLS playback support -->
  <script src="https://cdn.jsdelivr.net/npm/hls.js@1.4.0/dist/hls.min.js"></script>
</svelte:head>

<main class="shell">
  <section class="panel">
    <header class="panel-header">
      <div class="panel-title">
        <button type="button" class="back-btn" onclick={goBack}>← Back</button>
        <h1>{stream?.title ?? 'Loading...'}</h1>
        {#if stream}
          <p class="header-subtle">Duration: {formatDuration(stream.duration)}</p>
        {/if}
      </div>
    </header>

    {#if isLoading}
      <div class="player-placeholder">
        <p class="muted">Loading video...</p>
      </div>
    {:else if error}
      <div class="player-placeholder error-box">
        <p class="error" role="alert">{error}</p>
        <button class="retry-btn" onclick={() => window.location.reload()}>Retry</button>
      </div>
    {:else if stream}
      <div class="player-container">
        <video
          controls
          autoplay
          playsinline
          class="video-player"
          use:initializePlayer
          poster=""
        >
          <p>Your browser does not support the video tag.</p>
        </video>
      </div>
    {/if}
  </section>
</main>

<style>
  .shell {
    min-height: 100dvh;
    box-sizing: border-box;
    display: grid;
    justify-items: center;
    align-content: start;
    padding: 1rem 1rem 3rem;
  }

  .panel {
    width: min(46rem, 100%);
    background: linear-gradient(160deg, rgba(47, 51, 77, 0.95), rgba(34, 36, 54, 0.95));
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .panel-title {
    min-width: 0;
  }

  .back-btn {
    background: transparent;
    border: 1px solid rgba(162, 182, 217, 0.45);
    color: var(--fg);
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
    cursor: pointer;
  }

  .back-btn:hover {
    border-color: var(--accent);
    background: rgba(17, 26, 41, 0.72);
  }

  h1 {
    margin: 0.2rem 0 0;
    font-size: clamp(1.2rem, 3vw, 1.8rem);
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .header-subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
  }

  .player-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 300px;
    background: var(--surface);
    border-radius: 0.75rem;
    gap: 1rem;
  }

  .error-box {
    padding: 2rem;
    text-align: center;
  }

  .muted {
    margin: 0;
    color: var(--muted);
  }

  .error {
    margin: 0;
    color: var(--danger);
  }

  .retry-btn {
    background: var(--accent);
    color: #1e2030;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    font-weight: 600;
    cursor: pointer;
  }

  .player-container {
    position: relative;
    width: 100%;
    background: #000;
    border-radius: 0.75rem;
    overflow: hidden;
  }

  .video-player {
    width: 100%;
    max-height: 70vh;
    display: block;
  }

  /* Custom video controls styling */
  .video-player::-webkit-media-controls {
    background: rgba(0, 0, 0, 0.7);
  }

  .video-player::-webkit-media-controls-panel {
    background: rgba(0, 0, 0, 0.7);
  }
</style>
