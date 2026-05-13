<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getYouTubeRecentVideos, getYouTubeThumbnailUrl } from '$lib/api';
  import type { YoutubeVideo } from '$lib/api';
  import { formatTimeAgo } from '$lib/youtube/format';
  import { LoadedFade, YouTubeMediaRow } from '$lib/components/youtube';

  const DEFAULT_MAX_RESULTS = 25;

  let videos = $state<YoutubeVideo[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      videos = await getYouTubeRecentVideos(DEFAULT_MAX_RESULTS);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load recent videos';
    } finally {
      isLoading = false;
    }
  });

  function openVideo(videoId: string): void {
    if (typeof window !== 'undefined') {
      sessionStorage.setItem('youtubeBackContext', 'recent');
      sessionStorage.setItem('youtubeWatchReturnUrl', '/?youtube=recent');
    }
    goto(`/youtube/watch/${encodeURIComponent(videoId)}`);
  }

  function formatViewCount(count: number): string {
    if (count >= 1000000) {
      return `${(count / 1000000).toFixed(1)}M views`;
    }
    if (count >= 1000) {
      return `${(count / 1000).toFixed(1)}K views`;
    }
    return `${count} views`;
  }

  function formatDuration(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;

    if (hours > 0) {
      return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  }
</script>

  <div class="youtube-recent">
    {#if error}
      <p class="ui-error" role="alert">{error}</p>
    {:else if !isLoading && videos.length === 0}
      <p class="ui-muted">No recent videos found.</p>
    {:else if !isLoading}
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
</div>

<style>
  .youtube-recent {
    display: grid;
    gap: 1rem;
  }

  :global(.youtube-recent-row) {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
  }

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
</style>
