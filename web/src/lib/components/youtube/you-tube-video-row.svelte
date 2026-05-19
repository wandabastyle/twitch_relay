<script lang="ts">
  import { type YoutubeVideo, getYouTubeThumbnailUrl } from '$lib/api-client';
  import { formatDuration, formatViewCount } from '$lib/youtube/format';

  // Component props interface
  interface Props {
    /** Video data object */
    video: YoutubeVideo;
    /** Click handler for the row */
    onClick: () => void;
  }

  const { onClick, video }: Props = $props();
</script>

<button type="button" class="youtube-video-row" onclick={onClick}>
  <div class="youtube-video-thumb-wrap">
    <img
      class="youtube-video-thumb"
      src={getYouTubeThumbnailUrl(video.video_id)}
      alt={video.title}
      loading="lazy"
    />
    <span class="youtube-video-duration">{formatDuration(video.duration)}</span>
  </div>
  <div class="youtube-video-info">
    <h3 class="youtube-video-title" title={video.title}>{video.title}</h3>
    <div class="youtube-video-meta">
      {video.author} · {formatViewCount(video.view_count)} views · {video.published_text}
    </div>
    {#if video.description}
      <p class="youtube-video-description" title={video.description}>{video.description}</p>
    {/if}
  </div>
</button>

<style>
  .youtube-video-row {
    width: 100%;
    display: flex;
    align-items: stretch;
    gap: 1rem;
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--bg-soft) 62%, var(--surface));
    text-align: left;
    color: inherit;
    cursor: pointer;
    transition:
      border-color 0.2s ease,
      background-color 0.2s ease;
  }

  .youtube-video-row:hover,
  .youtube-video-row:focus-visible {
    border-color: var(--accent-border);
    background: var(--accent-soft);
    outline: none;
  }

  .youtube-video-row:focus-visible {
    box-shadow: 0 0 0 3px var(--focus-ring);
  }

  .youtube-video-title {
    margin: 0;
    font-weight: 600;
    font-size: 1rem;
    color: var(--fg);
    line-height: 1.3;
  }

  .youtube-video-meta {
    margin-top: 0.32rem;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .youtube-video-description {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: color-mix(in srgb, var(--fg) 80%, var(--muted));
    line-height: 1.4;
    opacity: 0.85;
    overflow: hidden;
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .youtube-video-thumb-wrap {
    position: relative;
    flex: 0 0 240px;
    max-width: 240px;
  }

  .youtube-video-info {
    min-width: 0;
    flex: 1;
  }

  .youtube-video-thumb {
    width: 100%;
    height: auto;
    aspect-ratio: 16 / 9;
    border-radius: 0.56rem;
    object-fit: cover;
    display: block;
    background: var(--surface-2);
  }

  .youtube-video-duration {
    position: absolute;
    right: 0.4rem;
    bottom: 0.4rem;
    padding: 0.15rem 0.35rem;
    border-radius: 0.25rem;
    background: rgba(0, 0, 0, 0.75);
    color: white;
    font-size: 0.8rem;
  }

  @media (max-width: 700px) {
    .youtube-video-row {
      flex-direction: column;
    }

    .youtube-video-thumb-wrap {
      flex-basis: auto;
      max-width: none;
      width: 100%;
    }
  }
</style>
