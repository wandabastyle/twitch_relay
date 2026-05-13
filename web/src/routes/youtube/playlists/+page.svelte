<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getYouTubePlaylists, getYouTubePlaylistThumbnailUrl } from '$lib/api';
  import type { YoutubePlaylist } from '$lib/api';
  import { formatTimeAgo } from '$lib/youtube/format';
  import { LoadedFade, YouTubeMediaRow, YouTubeShell } from '$lib/components/youtube';
  import { SkeletonMediaList, ErrorState, EmptyState } from '$lib/components/ui';

  let playlists = $state<YoutubePlaylist[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  async function loadPlaylists(): Promise<void> {
    isLoading = true;
    error = null;
    try {
      playlists = await getYouTubePlaylists();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load playlists';
    } finally {
      isLoading = false;
    }
  }

  onMount(loadPlaylists);

  function handlePlaylistClick(playlistId: string) {
    goto(`/youtube/playlist/${encodeURIComponent(playlistId)}`);
  }

  function getThumbnailUrl(playlist: YoutubePlaylist): string {
    return getYouTubePlaylistThumbnailUrl(playlist.playlist_id);
  }

  function getPlaylistInitial(title: string): string {
    return title.slice(0, 1).toUpperCase();
  }
</script>

<svelte:head>
  <title>Playlists - YouTube Relay</title>
</svelte:head>

  <YouTubeShell activeTab="playlists" subtitle="Your playlists">
  {#if isLoading}
    <SkeletonMediaList count={6} />
  {:else if error}
    <ErrorState
      message={error}
      onRetry={loadPlaylists}
      isRetrying={isLoading}
    />
  {:else if playlists.length === 0}
    <EmptyState
      title="No playlists found"
      description="Create playlists in YouTube to see them here."
      variant="playlists"
    />
  {:else}
    <LoadedFade loaded={true}>
      <div class="ui-list">
        {#each playlists as playlist (playlist.playlist_id)}
          {#snippet visual()}
            <div class="playlist-thumbnail-container">
              <img
                class="ui-thumbnail playlist-thumbnail"
                src={getThumbnailUrl(playlist)}
                alt={playlist.title}
                loading="lazy"
                onerror={(e) => {
                  const img = e.currentTarget as HTMLImageElement;
                  img.style.display = 'none';
                  const fallback = img.nextElementSibling as HTMLElement | null;
                  if (fallback) fallback.style.display = 'grid';
                }}
              />
              <div class="playlist-thumbnail-fallback" style="display: none;">
                {getPlaylistInitial(playlist.title)}
              </div>
            </div>
          {/snippet}

          {#snippet meta()}
            <span class="ui-media-meta playlist-meta">
              {playlist.video_count} videos · Updated {formatTimeAgo(playlist.updated)}
            </span>
          {/snippet}

          <YouTubeMediaRow
            title={playlist.title}
            onClick={() => handlePlaylistClick(playlist.playlist_id)}
            {visual}
            {meta}
            extraClass="youtube-playlist-row"
          />
        {/each}
      </div>
    </LoadedFade>
  {/if}
</YouTubeShell>

<style>
  .playlist-thumbnail-container {
    position: relative;
    width: 140px;
    height: 78px;
  }

  .playlist-thumbnail {
    width: 140px;
    height: 78px;
    border-radius: 0.5rem;
  }

  .playlist-thumbnail-fallback {
    position: absolute;
    top: 0;
    left: 0;
    width: 140px;
    height: 78px;
    border-radius: 0.5rem;
    display: none;
    place-items: center;
    text-transform: uppercase;
    font-weight: 700;
    font-size: 1.5rem;
    color: var(--fg);
    background: var(--surface-2);
  }

  :global(.youtube-playlist-row) {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
  }
</style>
