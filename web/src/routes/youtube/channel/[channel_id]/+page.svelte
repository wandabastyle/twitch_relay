<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { getYouTubeChannelVideos } from '$lib/api';
  import type { YoutubeVideo } from '$lib/api';

  let videos = $state<YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let channelName = $state('Channel');

  onMount(async () => {
    const channelId = $page.params.channel_id;
    if (!channelId) {
      error = 'No channel ID provided';
      isLoading = false;
      return;
    }

    try {
      const fetchedVideos = await getYouTubeChannelVideos(channelId);
      videos = fetchedVideos;
      if (fetchedVideos.length > 0) {
        channelName = fetchedVideos[0].author;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load videos';
    } finally {
      isLoading = false;
    }
  });

  function openVideo(videoId: string) {
    goto(`/youtube/watch/${encodeURIComponent(videoId)}`);
  }

  function goBack() {
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

<main class="shell">
  <section class="panel">
    <header class="panel-header">
      <div class="panel-title">
        <button type="button" class="back-btn" onclick={goBack}>← Back</button>
        <h1>{channelName}</h1>
        <p class="header-subtle">Latest Videos</p>
      </div>
    </header>

    {#if isLoading}
      <p class="muted">Loading videos...</p>
    {:else if error}
      <p class="error" role="alert">{error}</p>
    {:else if videos.length === 0}
      <p class="muted">No videos found for this channel.</p>
    {:else}
      <div class="videos-list">
        {#each videos as video (video.video_id)}
          <button
            type="button"
            class="video-row"
            onclick={() => openVideo(video.video_id)}
          >
            <div class="video-info">
              <span class="video-title" title={video.title}>{video.title}</span>
              <span class="video-channel">{video.author}</span>
              <span class="video-meta">
                {formatViewCount(video.view_count)} views • {video.published_text}
              </span>
              {#if video.description}
                <p class="video-description" title={video.description}>{video.description.slice(0, 120)}{video.description.length > 120 ? '...' : ''}</p>
              {/if}
            </div>
            <div class="video-thumbnail-wrap">
              <img
                class="video-thumbnail"
                src={video.thumbnail}
                alt={video.title}
                loading="lazy"
              />
              <span class="video-duration">{formatDuration(video.duration)}</span>
            </div>
          </button>
        {/each}
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
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  .header-subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
  }

  .muted {
    margin: 0;
    color: var(--muted);
  }

  .error {
    margin: 0 0 1rem;
    padding: 0.7rem 0.8rem;
    background: rgba(194, 67, 89, 0.18);
    border: 1px solid rgba(246, 135, 154, 0.45);
    border-radius: 0.6rem;
    color: color-mix(in srgb, var(--danger) 72%, white);
  }

  .videos-list {
    display: grid;
    gap: 1rem;
  }

  .video-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 1rem;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    background: var(--surface);
    cursor: pointer;
    text-align: left;
    align-items: start;
    transition: border-color 0.2s ease, background-color 0.2s ease;
  }

  .video-row:hover {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--surface) 90%, var(--accent));
  }

  .video-info {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }

  .video-title {
    font-weight: 600;
    font-size: 1rem;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .video-channel {
    font-size: 0.85rem;
    color: var(--accent);
    font-weight: 500;
  }

  .video-meta {
    font-size: 0.8rem;
    color: var(--muted);
  }

  .video-description {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: color-mix(in srgb, var(--fg) 70%, var(--muted));
    line-height: 1.4;
    overflow: hidden;
    display: box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .video-thumbnail-wrap {
    position: relative;
    flex-shrink: 0;
  }

  .video-thumbnail {
    width: 160px;
    height: 90px;
    border-radius: 0.5rem;
    object-fit: cover;
    background: var(--surface-2);
  }

  .video-duration {
    position: absolute;
    bottom: 0.3rem;
    right: 0.3rem;
    background: rgba(0, 0, 0, 0.8);
    color: white;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.15rem 0.4rem;
    border-radius: 0.25rem;
  }

  @media (max-width: 600px) {
    .video-row {
      grid-template-columns: 1fr;
      grid-template-rows: auto auto;
    }

    .video-thumbnail-wrap {
      order: -1;
    }

    .video-thumbnail {
      width: 100%;
      height: auto;
      aspect-ratio: 16 / 9;
    }
  }
</style>
