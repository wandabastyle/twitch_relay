<script lang="ts">
  import { onMount } from 'svelte';

  import { getYouTubeChannelVideos, refreshYouTubeChannelVideos, type YoutubeVideo } from '$lib/api-client';
  import { EmptyState, ErrorState, SkeletonVideoList } from '$lib/components/ui';
  import LoadedFade from '$lib/components/loaded-fade.svelte';
  import { navigate } from '$lib/router/router.svelte';
  import YouTubeVideoRow from '$lib/components/youtube/you-tube-video-row.svelte';

  const ZERO = 0;
  const DEFAULT_CHANNEL_NAME = 'Channel';
  const DEFAULT_SKELETON_COUNT = 6;
  const DEFAULT_TAB_TITLE = 'Channel Videos';
  const ERROR_NO_ID = 'No channel ID provided';
  const ERROR_NO_VIDEOS_DESC = "This channel doesn't have any videos available.";
  const ERROR_NO_VIDEOS_TITLE = 'No videos found';
  const ERROR_LOAD_FAILED = 'Failed to load videos';

  interface Props {
    channel_id: string;
  }

  const { channel_id }: Props = $props();

  let videos = $state<readonly YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string>();
  let channelName = $state(DEFAULT_CHANNEL_NAME);

  const returnUrl = $derived(`/youtube/channel/${channel_id}`);

  const loadChannelVideos = async (): Promise<void> => {
    if (!channel_id) {
      error = ERROR_NO_ID;
      isLoading = false;
      return;
    }

    isLoading = true;
    error = undefined;

    try {
      const { fromCache, videos: fetchedVideos } = await getYouTubeChannelVideos(channel_id);
      videos = fetchedVideos;
      if (fetchedVideos.length > ZERO) {
        channelName = fetchedVideos[ZERO].author;
      }
      isLoading = fromCache === false;

      if (!fromCache) {
        const refreshed = await refreshYouTubeChannelVideos(channel_id);
        videos = refreshed.videos;
        if (refreshed.videos.length > ZERO) {
          channelName = refreshed.videos[ZERO].author;
        }
      }
    } catch (error_) {
      const errorMessage = error_ instanceof Error ? error_.message : ERROR_LOAD_FAILED;
      error = errorMessage;
    } finally {
      isLoading = false;
    }
  };

  onMount(loadChannelVideos);

  const openVideo = (videoId: string): void => {
    navigate(`/youtube/watch/${encodeURIComponent(videoId)}`, {
      state: { youtubeReturnUrl: returnUrl },
    });
  };

  const goBack = (): void => {
    navigate('/youtube');
  };
</script>

<section class="ui-page-panel">
  <header class="panel-header">
    <div class="panel-title">
      <button type="button" class="ui-nav-chip" onclick={goBack}>Back</button>
      <h1 class="ui-page-title">{channelName}</h1>
      <p class="ui-page-subtle">Latest Videos</p>
    </div>
  </header>

  {#if isLoading}
    <SkeletonVideoList count={DEFAULT_SKELETON_COUNT} />
  {:else if error}
    <ErrorState
      message={error}
      onRetry={loadChannelVideos}
      isRetrying={isLoading}
    />
  {:else if videos.length === ZERO}
    <EmptyState
      title={ERROR_NO_VIDEOS_TITLE}
      description={ERROR_NO_VIDEOS_DESC}
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
</style>
