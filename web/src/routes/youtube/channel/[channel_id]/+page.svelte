<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { getYouTubeChannelVideos, refreshYouTubeChannelVideos, getYouTubeThumbnailUrl } from '$lib/api';
  import type { YoutubeVideo } from '$lib/api';
  import LoadedFade from '$lib/components/LoadedFade.svelte';

  let videos = $state<YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let channelName = $state('Channel');

  const returnUrl = $derived(`/youtube/channel/${$page.params.channel_id}`);

  onMount(async () => {
    const channelId = $page.params.channel_id;
    if (!channelId) {
      error = 'No channel ID provided';
      isLoading = false;
      return;
    }

    try {
      const result = await getYouTubeChannelVideos(channelId);
      videos = result.videos;
      if (result.videos.length > 0) {
        channelName = result.videos[0].author;
      }
      isLoading = result.fromCache === false;

      if (!result.fromCache) {
        const refreshed = await refreshYouTubeChannelVideos(channelId);
        videos = refreshed.videos;
        if (refreshed.videos.length > 0) {
          channelName = refreshed.videos[0].author;
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load videos';
    } finally {
      isLoading = false;
    }
  });

  function openVideo(videoId: string) {
    // Navigate to watch page with return URL in history state
    // This enables browser back to restore scroll position naturally
    goto(`/youtube/watch/${encodeURIComponent(videoId)}`, {
      state: { youtubeReturnUrl: returnUrl }
    });
  }

  function goBack() {
    goto('/youtube');
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

  function formatViewCount(count: number): string {
    if (count >= 1000000) {
      return `${(count / 1000000).toFixed(1)}M`;
    }
    if (count >= 1000) {
      return `${(count / 1000).toFixed(1)}K`;
    }
    return String(count);
  }
</script>

<svelte:head>
  <title>{channelName} - YouTube Relay</title>
</svelte:head>

<section class="ui-page-panel">
  <header class="panel-header">
    <div class="panel-title">
      <button type="button" class="ui-nav-chip" onclick={goBack}>Back</button>
      <h1 class="ui-page-title">{channelName}</h1>
      <p class="ui-page-subtle">Latest Videos</p>
    </div>
  </header>

  {#if error}
    <p class="ui-error" role="alert">{error}</p>
  {:else if !isLoading && videos.length === 0}
    <p class="ui-muted">No videos found for this channel.</p>
  {:else if !isLoading}
    <LoadedFade loaded={true}>
      <div class="youtube-video-list">
        {#each videos as video (video.video_id)}
          <button
            type="button"
            class="youtube-video-row"
            onclick={() => openVideo(video.video_id)}
          >
            <div class="youtube-video-thumb-wrap">
              <img
                class="youtube-video-thumb"
                src={getYouTubeThumbnailUrl(video.video_id)}
                alt={video.title}
                loading="lazy"
              />
              <span class="youtube-video-duration">{formatDuration(video.duration)}</span>
            </div>
            <div class="youtube-video-info">
              <h3 class="youtube-video-title" title={video.title}>{video.title}</h3>
              <div class="youtube-video-meta">
                {video.author} · {formatViewCount(video.view_count)} views · {video.published_text}
              </div>
              {#if video.description}
                <p class="youtube-video-description" title={video.description}>{video.description}</p>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    </LoadedFade>
  {/if}
</section>

<style>
  /* Width override: this page uses 46rem instead of default 42rem */
  .ui-page-panel {
    width: min(46rem, 100%);
  }

  /* Header layout - YouTube specific */
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

  .panel-title > .ui-nav-chip {
    margin-bottom: 0.5rem;
  }

  .ui-page-title {
    font-size: clamp(1.5rem, 4vw, 2rem);
  }

  .youtube-video-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .youtube-video-row {
    width: 100%;
    display: flex;
    align-items: stretch;
    gap: 1rem;
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--bg-soft) 62%, var(--surface));
    text-align: left;
    color: inherit;
    cursor: pointer;
    transition: border-color 0.2s ease, background-color 0.2s ease;
  }

  .youtube-video-row:hover,
  .youtube-video-row:focus-visible {
    border-color: var(--accent-border);
    background: var(--accent-soft);
    outline: none;
  }

  .youtube-video-row:focus-visible {
    box-shadow: 0 0 0 3px var(--focus-ring);
  }

  .youtube-video-title {
    margin: 0;
    font-weight: 600;
    font-size: 1rem;
    color: var(--fg);
    line-height: 1.3;
  }

  .youtube-video-meta {
    margin-top: 0.32rem;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .youtube-video-description {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: color-mix(in srgb, var(--fg) 80%, var(--muted));
    line-height: 1.4;
    opacity: 0.85;
    overflow: hidden;
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .youtube-video-thumb-wrap {
    position: relative;
    flex: 0 0 240px;
    max-width: 240px;
  }

  .youtube-video-info {
    min-width: 0;
    flex: 1;
  }

  .youtube-video-thumb {
    width: 100%;
    height: auto;
    aspect-ratio: 16 / 9;
    border-radius: 0.56rem;
    object-fit: cover;
    display: block;
    background: var(--surface-2);
  }

  .youtube-video-duration {
    position: absolute;
    right: 0.4rem;
    bottom: 0.4rem;
    padding: 0.15rem 0.35rem;
    border-radius: 0.25rem;
    background: rgba(0, 0, 0, 0.75);
    color: white;
    font-size: 0.8rem;
  }

  @media (max-width: 700px) {
    .youtube-video-row {
      flex-direction: column;
    }

    .youtube-video-thumb-wrap {
      flex-basis: auto;
      max-width: none;
      width: 100%;
    }
  }
</style>
