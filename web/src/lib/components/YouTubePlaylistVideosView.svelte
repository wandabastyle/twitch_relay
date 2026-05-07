<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getYouTubePlaylistVideos } from '$lib/api';
  import type { YoutubeVideo } from '$lib/api';

  interface Props {
    playlistId: string;
    playlistTitle?: string;
    onBack?: () => void;
  }

  let { playlistId, playlistTitle, onBack }: Props = $props();

  let videos = $state<YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      const result = await getYouTubePlaylistVideos(playlistId);
      videos = result.videos;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load playlist videos';
    } finally {
      isLoading = false;
    }
  });

  function formatDuration(seconds: number): string {
    if (!seconds) return '--:--';
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    if (hrs > 0) {
      return `${hrs}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function formatViewCount(count: number): string {
    if (count >= 1_000_000) {
      return `${(count / 1_000_000).toFixed(1)}M views`;
    }
    if (count >= 1_000) {
      return `${(count / 1_000).toFixed(1)}K views`;
    }
    return `${count} views`;
  }

  function openVideo(videoId: string) {
    goto(`/youtube/watch/${encodeURIComponent(videoId)}`);
  }

  function handleBack() {
    if (onBack) {
      onBack();
    }
  }
</script>

<div class="youtube-playlist-videos">
  <div class="playlist-videos-header">
    <button type="button" class="back-btn" onclick={handleBack}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M19 12H5M12 19l-7-7 7-7"/>
      </svg>
      Back
    </button>
    {#if playlistTitle}
      <span class="playlist-title-header">{playlistTitle}</span>
    {/if}
  </div>

  {#if isLoading}
    <p class="muted">Loading videos...</p>
  {:else if error}
    <p class="error" role="alert">{error}</p>
  {:else if videos.length === 0}
    <p class="muted">No videos found in this playlist.</p>
  {:else}
    <div class="videos-list">
      {#each videos as video (video.video_id)}
        <button
          type="button"
          class="video-row"
          onclick={() => openVideo(video.video_id)}
        >
          <div class="video-thumbnail-wrap">
            <img
              class="video-thumbnail"
              src={video.thumbnail}
              alt={video.title}
              loading="lazy"
            />
            <span class="video-duration">{formatDuration(video.duration)}</span>
          </div>
          <div class="video-main">
            <span class="video-title" title={video.title}>{video.title}</span>
            <span class="video-meta">
              {video.author} · {formatViewCount(video.view_count)} · {video.published_text}
            </span>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .youtube-playlist-videos {
    display: grid;
    gap: 1rem;
  }

  .playlist-videos-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.4rem 0.75rem;
    border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
    border-radius: 0.5rem;
    background: var(--surface);
    color: var(--fg);
    font-size: 0.9rem;
    cursor: pointer;
    transition: border-color 0.2s ease, background-color 0.2s ease;
  }

  .back-btn:hover,
  .back-btn:focus-visible {
    border-color: var(--accent-border);
    background: var(--accent-soft);
    outline: none;
  }

  .back-btn svg {
    width: 1rem;
    height: 1rem;
  }

  .playlist-title-header {
    font-weight: 600;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .videos-list {
    display: grid;
    gap: 0.75rem;
  }

  .video-row {
    display: grid;
    grid-template-columns: 160px minmax(0, 1fr);
    align-items: start;
    gap: 0.75rem;
    border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--bg-soft) 60%, var(--surface));
    padding: 0.8rem;
    text-align: left;
    color: inherit;
    cursor: pointer;
    transition: border-color 0.2s ease, background-color 0.2s ease;
  }

  .video-row:hover,
  .video-row:focus-visible {
    border-color: var(--accent-border);
    background: var(--accent-soft);
    outline: none;
  }

  .video-row:focus-visible {
    box-shadow: 0 0 0 3px var(--focus-ring);
  }

  .video-thumbnail-wrap {
    position: relative;
    width: 160px;
    height: 90px;
    border-radius: 0.5rem;
    overflow: hidden;
    background: var(--surface-2);
  }

  .video-thumbnail {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .video-duration {
    position: absolute;
    bottom: 0.35rem;
    right: 0.35rem;
    background: rgba(0, 0, 0, 0.8);
    color: white;
    font-size: 0.7rem;
    font-weight: 500;
    padding: 0.15rem 0.35rem;
    border-radius: 0.2rem;
  }

  .video-main {
    min-width: 0;
    display: grid;
    gap: 0.35rem;
    padding-top: 0.1rem;
  }

  .video-title {
    font-size: 0.92rem;
    font-weight: 600;
    text-align: left;
    color: var(--fg);
    line-height: 1.35;
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .video-meta {
    color: var(--muted);
    font-size: 0.78rem;
  }

  .muted {
    margin: 0;
    color: var(--muted);
  }

  .error {
    margin: 0;
    padding: 0.75rem;
    background: rgba(255, 82, 82, 0.15);
    border: 1px solid var(--danger);
    border-radius: 0.5rem;
    color: var(--danger);
  }
</style>
