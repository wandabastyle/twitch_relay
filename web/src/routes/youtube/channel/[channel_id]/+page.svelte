<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { getYouTubeChannelVideos, refreshYouTubeChannelVideos } from '$lib/api';
  import type { YoutubeVideo } from '$lib/api';
  import LoadedFade from '$lib/components/LoadedFade.svelte';
  import YouTubeVideoRow from '$lib/components/youtube/YouTubeVideoRow.svelte';
  import { SkeletonVideoList, ErrorState, EmptyState } from '$lib/components/ui';

  let videos = $state<YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let channelName = $state('Channel');

  const returnUrl = $derived(`/youtube/channel/${$page.params.channel_id}`);

  async function loadChannelVideos(): Promise<void> {
    const channelId = $page.params.channel_id;
    if (!channelId) {
      error = 'No channel ID provided';
      isLoading = false;
      return;
    }

    isLoading = true;
    error = null;

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
  }

  onMount(loadChannelVideos);

  function openVideo(videoId: string) {
    goto(`/youtube/watch/${encodeURIComponent(videoId)}`, {
      state: { youtubeReturnUrl: returnUrl }
    });
  }

  function goBack() {
    goto('/youtube');
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

  {#if isLoading}
    <SkeletonVideoList count={6} />
  {:else if error}
    <ErrorState
      message={error}
      onRetry={loadChannelVideos}
      isRetrying={isLoading}
    />
  {:else if videos.length === 0}
    <EmptyState
      title="No videos found"
      description="This channel doesn't have any videos available."
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
