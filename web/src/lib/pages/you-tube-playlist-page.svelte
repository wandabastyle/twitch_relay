<script lang="ts">
  import { onMount } from 'svelte';

  import { getYouTubePlaylistVideos, type YoutubeVideo } from '$lib/api-client';
  import { EmptyState, ErrorState, SkeletonVideoList } from '$lib/components/ui';
  import LoadedFade from '$lib/components/loaded-fade.svelte';
  import YouTubeVideoRow from '$lib/components/youtube/you-tube-video-row.svelte';
  import { navigate } from '$lib/router/router.svelte';

  const DEFAULT_ERROR_MESSAGE = 'Failed to load playlist videos';
  const MIN_VIDEOS_FOR_TITLE = 1;
  const NO_ID_ERROR = 'No playlist ID provided';
  const NO_VIDEOS_DESC = "This playlist doesn't contain any videos.";
  const NO_VIDEOS_TITLE = 'No videos found';
  const PLAYLIST_TITLE = 'Playlist';
  const PLAYLIST_VIDEOS_TITLE = 'Playlist Videos';
  const VIDEO_COUNT_IN_SKELETON = 6;

  interface Props {
    playlist_id: string;
  }

  const { playlist_id }: Props = $props();

  let videos = $state<readonly YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string>();
  let playlistTitle = $state(PLAYLIST_TITLE);

  const returnUrl = $derived(`/youtube/playlist/${playlist_id}`);

  const updatePlaylistTitle = (loadedVideos: readonly YoutubeVideo[]): void => {
    if (loadedVideos.length >= MIN_VIDEOS_FOR_TITLE) {
      playlistTitle = PLAYLIST_VIDEOS_TITLE;
    }
  };

  const loadPlaylist = async (): Promise<void> => {
    const { videos: loadedVideos } = await getYouTubePlaylistVideos(playlist_id);
    videos = loadedVideos;
    updatePlaylistTitle(loadedVideos);
  };

  const setLoadError = (catchError: unknown): void => {
    const errorMessage = catchError instanceof Error
      ? catchError.message
      : DEFAULT_ERROR_MESSAGE;
    error = errorMessage;
  };

  const loadPlaylistVideos = async (): Promise<void> => {
    if (!playlist_id) {
      error = NO_ID_ERROR;
      isLoading = false;
      return;
    }

    isLoading = true;
    error = undefined;

    try {
      await loadPlaylist();
    } catch (catchError) {
      setLoadError(catchError);
    } finally {
      isLoading = false;
    }
  };

  onMount(loadPlaylistVideos);

  const openVideo = (videoId: string): void => {
    navigate(`/youtube/watch/${encodeURIComponent(videoId)}`, {
      state: { youtubeReturnUrl: returnUrl },
    });
  };

  const goBack = (): void => {
    navigate('/youtube/playlists');
  };
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
    <SkeletonVideoList count={VIDEO_COUNT_IN_SKELETON} />
  {:else if typeof error === 'string'}
    <ErrorState
      message={error}
      onRetry={loadPlaylistVideos}
      isRetrying={isLoading}
    />
  {:else if videos.length === 0}
    <EmptyState
      title={NO_VIDEOS_TITLE}
      description={NO_VIDEOS_DESC}
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
