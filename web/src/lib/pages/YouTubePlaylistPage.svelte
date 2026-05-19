<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '$lib/router/router.svelte';
  import { getYouTubePlaylistVideos } from '$lib/api-client';
  import type { YoutubeVideo } from '$lib/api-client';
  import LoadedFade from '$lib/components/LoadedFade.svelte';
  import YouTubeVideoRow from '$lib/components/youtube/YouTubeVideoRow.svelte';
  import { SkeletonVideoList, ErrorState, EmptyState } from '$lib/components/ui';

  interface Props {
    playlist_id: string;
  }

  let { playlist_id }: Props = $props();

  let videos = $state<YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let playlistTitle = $state('Playlist');

  const returnUrl = $derived(`/youtube/playlist/${playlist_id}`);

  async function loadPlaylistVideos(): Promise<void> {
    if (!playlist_id) {
      error = 'No playlist ID provided';
      isLoading = false;
      return;
    }

    isLoading = true;
    error = null;

    try {
      const result = await getYouTubePlaylistVideos(playlist_id);
      videos = result.videos;
      if (result.videos.length > 0) {
        playlistTitle = 'Playlist Videos';
      }
      isLoading = false;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load playlist videos';
      isLoading = false;
    }
  }

  onMount(loadPlaylistVideos);

  function openVideo(videoId: string) {
    navigate(`/youtube/watch/${encodeURIComponent(videoId)}`, {
      state: { youtubeReturnUrl: returnUrl }
    });
  }

  function goBack() {
    navigate('/youtube/playlists');
  }
</script>

<section class="ui-page-panel">
  <header class="panel-header">
    <div class="panel-title">
      <button type="button" class="ui-nav-chip" onclick={goBack}>Back</button>
      <h1>{playlistTitle}</h1>
      <p class="header-subtle">{videos.length} videos</p>
    </div>
  </header>

  {#if isLoading}
    <SkeletonVideoList count={6} />
  {:else if error}
    <ErrorState
      message={error}
      onRetry={loadPlaylistVideos}
      isRetrying={isLoading}
    />
  {:else if videos.length === 0}
    <EmptyState
      title="No videos found"
      description="This playlist doesn't contain any videos."
      variant="videos"
    />
  {:else}
    <LoadedFade loaded={true}>
    <div class="youtube-video-list">
      {#each videos as video (video.video_id)}
        <YouTubeVideoRow {video} onClick={() => openVideo(video.video_id)} />
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

  .youtube-video-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
</style>
