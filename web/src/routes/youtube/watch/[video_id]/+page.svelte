<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { getYouTubeEmbedConfig, getYouTubeVideoMeta } from '$lib/api';

  const videoId = $derived($page.params.video_id ?? '');
  let embedUrl = $state('');
  let referrerPolicy = $state<'no-referrer' | 'strict-origin-when-cross-origin'>('no-referrer');
  let isLoading = $state(false);
  let error = $state<string | null>(null);
  let videoTitle = $state('YouTube video');
  let videoDuration = $state<number | null>(null);

  function buildEmbedUrl(
    baseUrl: string,
    id: string,
    defaults: { autoplay: number; quality: string; quality_dash: string },
  ): string {
    const params = new URLSearchParams({
      autoplay: String(defaults.autoplay),
      quality: defaults.quality,
      quality_dash: defaults.quality_dash,
    });
    return `${baseUrl}/embed/${encodeURIComponent(id)}?${params.toString()}`;
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
      embedUrl = buildEmbedUrl(config.invidious_base_url, videoId, config.defaults);
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
    if (videoId) {
      window.history.back();
      return;
    }
    goto('/');
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
        <button type="button" class="nav-chip-btn" onclick={goBack}>Back to videos</button>
        <h1>{videoTitle}</h1>
        {#if videoDuration !== null}
          <p class="subtle">Duration: {formatDuration(videoDuration)}</p>
        {/if}
      </div>
    </header>

    {#if error}
      <div class="player error-box">
        <p class="error" role="alert">{error}</p>
      </div>
    {:else if isLoading}
      <div class="player error-box">
        <p>Loading video...</p>
      </div>
    {:else if videoId && embedUrl}
      <iframe
        class="player"
        src={embedUrl}
        title="Invidious video player"
        allow="autoplay; fullscreen; encrypted-media; picture-in-picture"
        allowfullscreen
        loading="eager"
        referrerpolicy={referrerPolicy}
      ></iframe>
    {:else}
      <div class="player error-box">
        <p class="error" role="alert">Unable to initialize player.</p>
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
    aspect-ratio: 16 / 9;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: #000;
  }

  .error-box {
    min-height: 16rem;
    display: grid;
    place-items: center;
    padding: 1rem;
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
</style>
