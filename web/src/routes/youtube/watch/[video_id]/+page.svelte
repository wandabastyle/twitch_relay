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
  let retryCount = $state(0);
  let isRetrying = $state(false);

  const MAX_RETRIES = 3;
  const RETRY_DELAY_MS = 1000;
  const HLS_MANIFEST_TIMEOUT_MS = 8000;

  async function delay(ms: number): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, ms));
  }

  function cleanupPlayer() {
    if (hls) {
      hls.destroy();
      hls = null;
    }
    if (player) {
      player.pause();
      player.removeAttribute('src');
      // Note: Do NOT call player.load() here - it triggers a spurious
      // MEDIA_ERR_SRC_NOT_SUPPORTED error that causes overlapping retry calls
    }
  }

  function setLoadingComplete() {
    isLoading = false;
  }

  function applyStreamToPlayer(node: HTMLVideoElement, nextStream: VideoStream): Promise<boolean> {
    return new Promise((resolve) => {
      if (nextStream.is_hls) {
        if (typeof window !== 'undefined' && (window as any).Hls) {
          const Hls = (window as any).Hls;
          if (Hls.isSupported()) {
            hls = new Hls({
              maxBufferLength: 30,
              maxMaxBufferLength: 60,
            });
            hls.loadSource(nextStream.stream_url);
            hls.attachMedia(node);

            // Set up manifest parsed handler - success case
            const onManifestParsed = () => {
              setLoadingComplete();
              // Reset retry count on successful playback start
              retryCount = 0;
              node.play().catch(() => {
                // Autoplay prevented, user needs to interact
              });
              resolve(true);
            };

            // Set up timeout for manifest loading
            const manifestTimeout = setTimeout(() => {
              hls.off(Hls.Events.MANIFEST_PARSED, onManifestParsed);
              resolve(false);
            }, HLS_MANIFEST_TIMEOUT_MS);

            hls.on(Hls.Events.MANIFEST_PARSED, () => {
              clearTimeout(manifestTimeout);
              onManifestParsed();
            });

            hls.on(Hls.Events.ERROR, async (_: any, data: any) => {
              if (data.fatal) {
                clearTimeout(manifestTimeout);
                hls.off(Hls.Events.MANIFEST_PARSED, onManifestParsed);

                try {
                  const videoId = $page.params.video_id;
                  if (!videoId || !(await retryResolve(videoId))) {
                    error = 'Failed to load video stream';
                    isLoading = false;
                    resolve(false);
                  }
                } catch {
                  error = 'Failed to load video stream';
                  isLoading = false;
                  resolve(false);
                }
              }
            });
          } else if (node.canPlayType('application/vnd.apple.mpegurl')) {
            node.src = nextStream.stream_url;
            // For native HLS, we can't easily detect success, assume it worked
            // The video element's error event will catch failures
            resolve(true);
          } else {
            error = 'HLS playback not supported in this browser';
            isLoading = false;
            resolve(false);
          }
        } else if (node.canPlayType('application/vnd.apple.mpegurl')) {
          node.src = nextStream.stream_url;
          resolve(true);
        } else {
          error = 'HLS playback not supported in this browser';
          isLoading = false;
          resolve(false);
        }
      } else {
        node.src = nextStream.stream_url;
        resolve(true);
      }
    });
  }

  async function resolveStream(videoId: string, retryAttempt?: number): Promise<VideoStream> {
    return resolveYouTubeVideo({
      video_id: videoId,
      retry_attempt: retryAttempt,
    });
  }

  async function retryResolve(videoId: string): Promise<boolean> {
    // Concurrency guard: prevent overlapping retry calls
    if (isRetrying) {
      return false;
    }

    isRetrying = true;

    try {
      while (retryCount < MAX_RETRIES) {
        retryCount += 1;
        isLoading = true;
        error = null;
        cleanupPlayer();

        await delay(RETRY_DELAY_MS);

        try {
          const resolved = await resolveStream(videoId, retryCount);
          stream = resolved;
          if (player) {
            const applied = await applyStreamToPlayer(player, resolved);
            if (applied) {
              // Success - stream resolved and applied
              return true;
            }
            // applyStreamToPlayer returned false, meaning manifest failed to load
            // This counts as a failed attempt, continue to next retry
            if (retryCount >= MAX_RETRIES) {
              error = 'Failed to load video stream';
              isLoading = false;
              return false;
            }
          } else {
            // No player available, can't apply stream
            error = 'Player not available';
            isLoading = false;
            return false;
          }
        } catch (e) {
          // This retry failed - continue to next iteration if retries remain
          if (retryCount >= MAX_RETRIES) {
            error = 'Failed to load video stream';
            isLoading = false;
            return false;
          }
        }
      }
      // All retries exhausted
      error = 'Failed to load video stream';
      isLoading = false;
      return false;
    } finally {
      isRetrying = false;
    }
  }

  onMount(async () => {
    const videoId = $page.params.video_id;
    if (!videoId) {
      error = 'No video ID provided';
      isLoading = false;
      return;
    }

    try {
      const resolved = await resolveStream(videoId);
      stream = resolved;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load video';
    } finally {
      isLoading = false;
    }
  });

  onDestroy(() => {
    cleanupPlayer();
  });

  function initializePlayer(node: HTMLVideoElement) {
    player = node;

    if (!stream) return;

    applyStreamToPlayer(node, stream);

    node.addEventListener('play', () => {
      isPlaying = true;
    });

    node.addEventListener('pause', () => {
      isPlaying = false;
    });

    // Reset retry count when playback successfully starts
    const onPlaybackSuccess = () => {
      retryCount = 0;
      setLoadingComplete();
    };

    node.addEventListener('loadedmetadata', onPlaybackSuccess);
    node.addEventListener('canplay', onPlaybackSuccess);

    const onVideoError = async () => {
      // Ignore errors if no src is set (spurious error from cleanupPlayer)
      if (!node.src && !stream) {
        return;
      }

      try {
        const videoId = $page.params.video_id;
        if (!videoId) {
          error = 'No video ID available';
          isLoading = false;
          return;
        }

        const didRetry = await retryResolve(videoId);
        if (!didRetry) {
          // retryResolve will have set error if all retries failed
          // But if it returned false due to concurrent call, we shouldn't show error
          if (!isRetrying && retryCount >= MAX_RETRIES) {
            error = 'Failed to load video stream';
            isLoading = false;
          }
        }
      } catch {
        if (!isRetrying) {
          error = 'Failed to load video stream';
          isLoading = false;
        }
      }
    };

    node.addEventListener('error', onVideoError);

    return {
      destroy() {
        node.removeEventListener('error', onVideoError);
        node.removeEventListener('loadedmetadata', onPlaybackSuccess);
        node.removeEventListener('canplay', onPlaybackSuccess);
      },
    };
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
    <header class="player-header">
      <div>
        <button type="button" class="nav-chip-btn" onclick={goBack}>Back to videos</button>
        <h1>{stream?.title ?? 'Loading...'}</h1>
        {#if stream}
          <p class="subtle">Duration: {formatDuration(stream.duration)}</p>
        {/if}
      </div>
    </header>

    {#if isLoading}
      <div class="player loading">
        <p class="muted">Loading video...</p>
      </div>
    {:else if error}
      <div class="player loading error-box">
        <p class="error" role="alert">{error}</p>
        <button class="retry-btn" onclick={() => window.location.reload()}>Retry</button>
      </div>
    {:else if stream}
      <video
        controls
        autoplay
        playsinline
        class="player"
        use:initializePlayer
        poster=""
      >
        <p>Your browser does not support the video tag.</p>
      </video>
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
    width: min(74rem, 96vw);
    background: color-mix(in srgb, var(--surface) 82%, var(--bg-soft));
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
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

  .nav-chip-btn {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 0.6rem;
    color: var(--fg);
    padding: 0.4rem 0.8rem;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    line-height: 1;
    margin-bottom: 0.5rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 2rem;
  }

  .nav-chip-btn:hover {
    border-color: var(--accent-border);
    background: var(--accent-soft);
  }

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

  .player {
    width: 100%;
    height: auto;
    border-radius: 0;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: #000;
    min-height: 16rem;
  }

  .player.loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    color: var(--muted);
  }

  .error-box {
    text-align: center;
    padding: 1rem;
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
    color: #fff;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    font-weight: 600;
    cursor: pointer;
  }

  .retry-btn:hover {
    background: var(--accent-hover, var(--accent));
  }

  @media (min-width: 1100px) {
    .shell {
      padding: 0.75rem 1rem;
    }
  }
</style>
