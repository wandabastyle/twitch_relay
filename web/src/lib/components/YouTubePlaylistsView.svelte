<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getYouTubePlaylists, getYouTubePlaylistThumbnailUrl } from '$lib/api';
  import type { YoutubePlaylist } from '$lib/api';

  let playlists = $state<YoutubePlaylist[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      playlists = await getYouTubePlaylists();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load playlists';
    } finally {
      isLoading = false;
    }
  });

  function formatTimeAgo(timestamp: number): string {
    if (!timestamp) return '';
    const seconds = Math.floor(Date.now() / 1000) - timestamp;
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(seconds / 3600);
    const days = Math.floor(seconds / 86400);
    const weeks = Math.floor(days / 7);
    const months = Math.floor(days / 30);
    const years = Math.floor(days / 365);

    if (years > 0) return `${years}y ago`;
    if (months > 0) return `${months}mo ago`;
    if (weeks > 0) return `${weeks}w ago`;
    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    if (minutes > 0) return `${minutes}m ago`;
    return 'Just now';
  }

  function handlePlaylistClick(playlistId: string) {
    goto(`/youtube/playlist/${encodeURIComponent(playlistId)}`);
  }

  function getThumbnailUrl(playlist: YoutubePlaylist): string {
    return getYouTubePlaylistThumbnailUrl(playlist.playlist_id);
  }

</script>

<div class="youtube-playlists">
  {#if isLoading}
    <p class="muted">Loading playlists...</p>
  {:else if error}
    <p class="error" role="alert">{error}</p>
  {:else if playlists.length === 0}
    <p class="muted">No playlists found.</p>
  {:else}
    <div class="playlists-list">
      {#each playlists as playlist (playlist.playlist_id)}
        <button
          type="button"
          class="playlist-row"
          onclick={() => handlePlaylistClick(playlist.playlist_id)}
        >
          <div class="playlist-thumbnail-wrap">
            <img
              class="playlist-thumbnail"
              src={getThumbnailUrl(playlist)}
              alt={playlist.title}
              loading="lazy"
            />
          </div>
          <div class="playlist-main">
            <span class="playlist-title" title={playlist.title}>{playlist.title}</span>
            <span class="playlist-meta">
              {playlist.video_count} videos · Updated {formatTimeAgo(playlist.updated)}
            </span>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .youtube-playlists {
    display: grid;
    gap: 1rem;
  }

  .playlists-list {
    display: grid;
    gap: 0.75rem;
  }

  .playlist-row {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
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

  .playlist-row:hover,
  .playlist-row:focus-visible {
    border-color: var(--accent-border);
    background: var(--accent-soft);
    outline: none;
  }

  .playlist-row:focus-visible {
    box-shadow: 0 0 0 3px var(--focus-ring);
  }

  .playlist-thumbnail-wrap {
    height: 100%;
    min-height: 78px;
    display: flex;
    align-items: center;
  }

  .playlist-thumbnail {
    width: 140px;
    height: 78px;
    border-radius: 0.5rem;
    object-fit: cover;
    display: block;
    background: var(--surface-2);
  }

  .playlist-main {
    min-width: 0;
    display: grid;
    gap: 0.28rem;
  }

  .playlist-title {
    font-size: 0.95rem;
    font-weight: 600;
    text-align: left;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .playlist-meta {
    color: var(--muted);
    font-size: 0.8rem;
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
