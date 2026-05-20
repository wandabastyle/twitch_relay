<script lang="ts">
import SkeletonText from './skeleton-text.svelte';
import SkeletonThumbnail from './skeleton-thumbnail.svelte';

  const DEFAULT_COUNT = 5;

  interface Props {
    count?: number;
  }

  const { count = DEFAULT_COUNT }: Props = $props();
</script>

<div class="skeleton-video-list">
  {#each Array(count) as _}
    <div class="skeleton-video-row">
      <SkeletonThumbnail width="240px" aspectRatio="16 / 9" />
      <div class="skeleton-video-info">
        <SkeletonText lines={2} width="85%" />
      </div>
    </div>
  {/each}
</div>

<style>
  .skeleton-video-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .skeleton-video-row {
    width: 100%;
    box-sizing: border-box;
    display: flex;
    align-items: stretch;
    gap: 1rem;
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--bg-soft) 62%, var(--surface));
  }

  .skeleton-video-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  @media (max-width: 700px) {
    .skeleton-video-row {
      flex-direction: column;
    }
  }
</style>
