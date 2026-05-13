<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import ArrowLeftRight from 'lucide-svelte/icons/arrow-left-right';
  import { getYouTubeRecentVideos, getYouTubeThumbnailUrl } from '$lib/api';
  import type { YoutubeVideo } from '$lib/api';
  import { formatTimeAgo } from '$lib/youtube/format';
  import { LoadedFade, YouTubeMediaRow } from '$lib/components/youtube';

  const DEFAULT_MAX_RESULTS = 25;

  let videos = $state<YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      videos = await getYouTubeRecentVideos(DEFAULT_MAX_RESULTS);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load recent videos';
    } finally {
      isLoading = false;
    }
  });

  function openVideo(videoId: string): void {
    goto(`/youtube/watch/${encodeURIComponent(videoId)}`);
  }

  function formatViewCount(count: number): string {
    if (count >= 1000000) {
      return `${(count / 1000000).toFixed(1)}M views`;
    }
    if (count >= 1000) {
      return `${(count / 1000).toFixed(1)}K views`;
    }
    return `${count} views`;
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
  <title>Recent - YouTube Relay</title>
</svelte:head>

<section class="youtube-panel">
  <header class="panel-header">
    <div class="panel-title">
      <p class="eyebrow">Private Deck</p>
      <button
        type="button"
        class="relay-title-button"
        onclick={() => goto('/twitch')}
        aria-label="Switch to Twitch Relay"
        title="Switch to Twitch Relay"
      >
        <h1>YouTube Relay</h1>
        <span class="toggle-icon" aria-hidden="true">
          <ArrowLeftRight size={14} />
        </span>
      </button>
      <p class="header-subtle">Recent videos from subscriptions</p>
    </div>
  </header>

  <nav class="youtube-nav">
    <a href="/youtube" class="nav-link">Subscriptions</a>
    <a href="/youtube/recent" class="nav-link active">Recent</a>
    <a href="/youtube/playlists" class="nav-link">Playlists</a>
  </nav>

  {#if error}
    <p class="ui-error" role="alert">{error}</p>
  {:else if !isLoading && videos.length === 0}
    <p class="ui-muted">No recent videos found.</p>
  {:else if !isLoading}
    <LoadedFade loaded={true}>
      <div class="ui-list">
        {#each videos as video (video.video_id)}
          {#snippet visual()}
            <div class="recent-thumbnail-wrap">
              <img
                class="ui-thumbnail recent-thumbnail"
                src={getYouTubeThumbnailUrl(video.video_id)}
                alt={video.title}
                loading="lazy"
              />
              <span class="recent-duration">{formatDuration(video.duration)}</span>
            </div>
          {/snippet}

          {#snippet meta()}
            <span class="ui-media-meta recent-meta">
              {video.author} · {formatViewCount(video.view_count)} · {formatTimeAgo(video.published)}
            </span>
          {/snippet}

          <YouTubeMediaRow
            title={video.title}
            onClick={() => openVideo(video.video_id)}
            {visual}
            {meta}
            extraClass="youtube-recent-row"
          />
        {/each}
      </div>
    </LoadedFade>
  {/if}
</section>

<style>
  .youtube-panel {
    width: min(46rem, 100%);
    background: linear-gradient(160deg, color-mix(in srgb, var(--surface) 95%, transparent), color-mix(in srgb, var(--bg-soft) 95%, transparent));
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
    position: relative;
  }

  .panel-title {
    min-width: 0;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .relay-title-button,
  .relay-title-button:hover,
  .relay-title-button:focus,
  .relay-title-button:active {
    text-decoration: none;
  }

  .relay-title-button {
    appearance: none;
    background: transparent;
    border: 0;
    padding: 0;
    margin: 0.2rem 0 0;
    font: inherit;
    font-weight: inherit;
    cursor: pointer;
    text-align: left;
    color: inherit;
    display: inline-flex;
    align-items: baseline;
    gap: 0.4rem;
  }

  .relay-title-button h1 {
    margin: 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  .toggle-icon {
    display: inline-flex;
    align-items: center;
    opacity: 0.45;
    transition: opacity 0.15s ease, transform 0.15s ease;
    color: var(--muted);
  }

  .relay-title-button:hover .toggle-icon {
    opacity: 0.9;
    color: var(--accent);
    transform: rotate(180deg);
  }

  .relay-title-button:hover {
    color: var(--accent);
  }

  .header-subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
  }

  .youtube-nav {
    display: flex;
    gap: 1.25rem;
    margin-bottom: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  }

  .nav-link {
    color: var(--muted);
    text-decoration: none;
    font-size: 0.9rem;
    font-weight: 500;
    padding: 0.35rem 0;
    border-bottom: 2px solid transparent;
    transition: color 0.2s ease, border-color 0.2s ease;
  }

  .nav-link:hover {
    color: var(--fg);
  }

  .nav-link.active {
    color: var(--fg);
    border-bottom-color: var(--accent);
  }

  .recent-thumbnail-wrap {
    position: relative;
    width: 140px;
    height: 78px;
  }

  .recent-thumbnail {
    width: 140px;
    height: 78px;
    border-radius: 0.5rem;
  }

  .recent-duration {
    position: absolute;
    right: 0.35rem;
    bottom: 0.35rem;
    padding: 0.1rem 0.3rem;
    border-radius: 0.25rem;
    font-size: 0.74rem;
    line-height: 1.1;
    color: white;
    background: rgba(0, 0, 0, 0.76);
  }

  .recent-meta {
    line-height: 1.35;
  }

  :global(.youtube-recent-row) {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
  }
</style>
