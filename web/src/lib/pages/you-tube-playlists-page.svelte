<script lang="ts">
  import { onMount } from 'svelte';

  import { getYouTubePlaylistThumbnailUrl, getYouTubePlaylists, type YoutubePlaylist } from '$lib/api-client';
  import { EmptyState, ErrorState, SkeletonMediaList } from '$lib/components/ui';
  import { LoadedFade, YouTubeMediaRow, YouTubeShell } from '$lib/components/youtube';
  import { navigate } from '$lib/router/router.svelte';
  import { formatTimeAgo } from '$lib/youtube/format';

  const FAILED_TO_LOAD = 'Failed to load playlists';
  const INITIAL_SLICE_INDEX = 0;
  const INITIAL_SLICE_LENGTH = 1;
  const NO_PLAYLISTS_DESC = 'Create playlists in YouTube to see them here.';
  const NO_PLAYLISTS_TITLE = 'No playlists found';
  const SKELETON_COUNT = 6;

  let playlists = $state<YoutubePlaylist[]>([]);
  let isLoading = $state(true);
  let error = $state<string | undefined>(undefined);

  const loadPlaylists = async (): Promise<void> => {
    isLoading = true;
    error = undefined;
    try {
      playlists = await getYouTubePlaylists();
    } catch (error_) {
      const errorMessage = error_ instanceof Error ? error_.message : FAILED_TO_LOAD;
      error = errorMessage;
    } finally {
      isLoading = false;
    }
  };

  onMount(loadPlaylists);

  const handlePlaylistClick = (playlistId: string): void => {
    navigate(`/youtube/playlist/${encodeURIComponent(playlistId)}`);
  };

  const getThumbnailUrl = (playlist: YoutubePlaylist): string => getYouTubePlaylistThumbnailUrl(playlist.playlist_id);

  const getPlaylistInitial = (title: string): string => title.slice(INITIAL_SLICE_INDEX, INITIAL_SLICE_LENGTH).toUpperCase();
</script>

<YouTubeShell activeTab="playlists" subtitle="Your playlists">
  {#if isLoading}
    <SkeletonMediaList count={SKELETON_COUNT} />
  {:else if error}
    <ErrorState
      message={error}
      onRetry={loadPlaylists}
      isRetrying={isLoading}
    />
  {:else if playlists.length === 0}
    <EmptyState
      title={NO_PLAYLISTS_TITLE}
      description={NO_PLAYLISTS_DESC}
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
                  if (fallback) {
                    fallback.style.display = 'grid';
                  }
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
