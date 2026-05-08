<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { getYouTubeEmbedConfig, getYouTubeVideoMeta } from '$lib/api';
  import AppVersion from '$lib/components/AppVersion.svelte';

  const videoId = $derived($page.params.video_id ?? '');
  let embedUrl = $state('');
  let referrerPolicy = $state<'no-referrer' | 'strict-origin-when-cross-origin'>('no-referrer');
  let isLoading = $state(false);
  let error = $state<string | null>(null);
  let videoTitle = $state('YouTube video');
  let videoDuration = $state<number | null>(null);

  function buildEmbedUrl(
    id: string,
    defaults: { autoplay: number; quality: string; quality_dash: string },
  ): string {
    const params = new URLSearchParams({
      autoplay: String(defaults.autoplay),
      quality: defaults.quality,
      quality_dash: defaults.quality_dash,
    });

    // Use backend proxy endpoint to avoid Basic auth popup
    return `/api/youtube/embed/${encodeURIComponent(id)}?${params.toString()}`;
  }

  onMount(async () => {
    if (!videoId) {
      error = 'No video ID provided.';
      return;
    }

    isLoading = true;
    error = null;

    try {
      const [config, meta] = await Promise.all([getYouTubeEmbedConfig(), getYouTubeVideoMeta(videoId)]);
      embedUrl = buildEmbedUrl(videoId, config.defaults);
      videoTitle = meta.title;
      videoDuration = meta.duration;
      referrerPolicy =
        config.referrer_policy === 'strict-origin-when-cross-origin'
          ? 'strict-origin-when-cross-origin'
          : 'no-referrer';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load embed configuration.';
    } finally {
      isLoading = false;
    }
  });

  function goBack() {
    // Check if we have a stored return URL from sessionStorage
    if (typeof window !== 'undefined') {
      const returnUrl = sessionStorage.getItem('youtubeWatchReturnUrl');
      if (returnUrl) {
        sessionStorage.removeItem('youtubeWatchReturnUrl');
        goto(returnUrl);
        return;
      }
    }

    // Fallback to context-based navigation with new URL pattern
    const context = sessionStorage.getItem('youtubeBackContext');
    if (context) {
      sessionStorage.removeItem('youtubeBackContext');
      if (context === 'playlists') {
        goto('/?youtube=playlists');
        return;
      } else {
        goto('/?youtube=subscriptions');
        return;
      }
    }

    // Ultimate fallback
    if (window.history.length > 1) {
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
  <title>{videoId ? `${videoTitle} - YouTube Relay` : 'YouTube Relay'}</title>
</svelte:head>

<main class="shell">
  <section class="panel">
    <header class="player-header">
      <div>
        <button type="button" class="ui-nav-chip" onclick={goBack}>Back to videos</button>
        <h1>{videoTitle}</h1>
        {#if videoDuration !== null}
          <p class="subtle">Duration: {formatDuration(videoDuration)}</p>
        {/if}
      </div>
    </header>

    {#if error}
      <div class="player-wrapper">
        <div class="player error-box">
          <p class="error" role="alert">{error}</p>
        </div>
      </div>
    {:else if isLoading}
      <div class="player-wrapper">
        <div class="player loading-box">
          <p>Loading video...</p>
        </div>
      </div>
    {:else if videoId && embedUrl}
      <div class="player-wrapper">
        <iframe
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
          <p class="error" role="alert">Unable to initialize player.</p>
        </div>
      </div>
    {/if}
  </section>
  <AppVersion />
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

  .error {
    margin: 0;
    color: var(--danger);
  }

  @media (min-width: 1100px) {
    .shell {
      padding: 0.75rem 1rem;
    }
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
