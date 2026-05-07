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
    // Store context before navigating
    if (typeof window !== 'undefined') {
      sessionStorage.setItem('youtubeBackContext', 'playlists');
    }
    goto(`/youtube/playlist/${encodeURIComponent(playlistId)}`);
  }

  function getThumbnailUrl(playlist: YoutubePlaylist): string {
    return getYouTubePlaylistThumbnailUrl(playlist.playlist_id);
  }

</script>

<div class="youtube-playlists">
  {#if isLoading}
    <p class="ui-muted">Loading playlists...</p>
  {:else if error}
    <p class="ui-error" role="alert">{error}</p>
  {:else if playlists.length === 0}
    <p class="ui-muted">No playlists found.</p>
  {:else}
    <div class="ui-list">
      {#each playlists as playlist (playlist.playlist_id)}
        <button
          type="button"
          class="ui-card ui-card-interactive playlist-row"
          onclick={() => handlePlaylistClick(playlist.playlist_id)}
        >
          <div class="ui-media-visual playlist-thumbnail-wrap">
            <img
              class="ui-thumbnail playlist-thumbnail"
              src={getThumbnailUrl(playlist)}
              alt={playlist.title}
              loading="lazy"
            />
          </div>
          <div class="ui-media-main playlist-main">
            <span class="ui-media-title playlist-title" title={playlist.title}>{playlist.title}</span>
            <span class="ui-media-meta playlist-meta">
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

  /* Component-specific layout and sizing for shared classes */
  .playlist-row {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
  }

  .playlist-thumbnail-wrap {
    height: 100%;
    min-height: 78px;
  }

  .playlist-thumbnail {
    width: 140px;
    height: 78px;
    border-radius: 0.5rem;
  }

  /* .muted, .error, .playlists-list, .playlist-row base styles,
     .playlist-thumbnail-wrap base, .playlist-thumbnail base (except sizing),
     .playlist-main, .playlist-title, .playlist-meta now provided by app.css via .ui-* classes */
</style>
