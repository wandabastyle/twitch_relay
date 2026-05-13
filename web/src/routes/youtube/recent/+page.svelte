<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getYouTubeRecentVideos, getYouTubeThumbnailUrl } from '$lib/api';
  import type { YoutubeVideo } from '$lib/api';
  import { formatTimeAgo, formatDuration, formatViewCount } from '$lib/youtube/format';
  import { LoadedFade, YouTubeMediaRow, YouTubeShell } from '$lib/components/youtube';
  import { SkeletonVideoList, ErrorState, EmptyState } from '$lib/components/ui';

  const DEFAULT_MAX_RESULTS = 25;

  let videos = $state<YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  async function loadRecentVideos(): Promise<void> {
    isLoading = true;
    error = null;
    try {
      videos = await getYouTubeRecentVideos(DEFAULT_MAX_RESULTS);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load recent videos';
    } finally {
      isLoading = false;
    }
  }

  onMount(loadRecentVideos);

  function openVideo(videoId: string): void {
    goto(`/youtube/watch/${encodeURIComponent(videoId)}`, {
      state: { youtubeReturnUrl: '/youtube/recent' }
    });
  }
</script>

<svelte:head>
  <title>Recent - YouTube Relay</title>
</svelte:head>

  <YouTubeShell activeTab="recent" subtitle="Recent videos from subscriptions">
  {#if isLoading}
    <SkeletonVideoList count={6} />
  {:else if error}
    <ErrorState
      message={error}
      onRetry={loadRecentVideos}
      isRetrying={isLoading}
    />
  {:else if videos.length === 0}
    <EmptyState
      title="No recent videos found"
      description="Recent videos from your subscriptions will appear here."
      variant="videos"
    />
  {:else}
    <LoadedFade loaded={true}>
      <div class="ui-list">
        {#each videos as video (video.video_id)}
          {#snippet visual()}
            <div class="recent-thumbnail-wrap">
              <img
                class="ui-thumbnail recent-thumbnail"
                src={getYouTubeThumbnailUrl(video.video_id)}
                alt={video.title}
                loading="lazy"
              />
              <span class="recent-duration">{formatDuration(video.duration)}</span>
            </div>
          {/snippet}

          {#snippet meta()}
            <span class="ui-media-meta recent-meta">
              {video.author} · {formatViewCount(video.view_count)} · {formatTimeAgo(video.published)}
            </span>
          {/snippet}

          <YouTubeMediaRow
            title={video.title}
            onClick={() => openVideo(video.video_id)}
            {visual}
            {meta}
            extraClass="youtube-recent-row"
          />
        {/each}
      </div>
    </LoadedFade>
  {/if}
</YouTubeShell>

<style>
  .recent-thumbnail-wrap {
    position: relative;
    width: 140px;
    height: 78px;
  }

  .recent-thumbnail {
    width: 140px;
    height: 78px;
    border-radius: 0.5rem;
  }

  .recent-duration {
    position: absolute;
    right: 0.35rem;
    bottom: 0.35rem;
    padding: 0.1rem 0.3rem;
    border-radius: 0.25rem;
    font-size: 0.74rem;
    line-height: 1.1;
    color: white;
    background: rgba(0, 0, 0, 0.76);
  }

  .recent-meta {
    line-height: 1.35;
  }

  :global(.youtube-recent-row) {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
  }
</style>
