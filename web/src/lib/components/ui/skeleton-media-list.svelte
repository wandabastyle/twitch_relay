<script lang="ts">
import SkeletonText from './skeleton-text.svelte';

import SkeletonThumbnail from './skeleton-thumbnail.svelte';

  const DEFAULT_COUNT = 8;
  const DEFAULT_AVATAR_SIZE = "74px";

  interface Props {
    count?: number;
    avatarSize?: string;
  }

  const { count = DEFAULT_COUNT, avatarSize = DEFAULT_AVATAR_SIZE }: Props = $props();
</script>

<div class="skeleton-media-list">
  {#each Array(count) as _}
    <div class="skeleton-media-row">
      <SkeletonThumbnail width={avatarSize} height={avatarSize} aspectRatio="1 / 1" borderRadius="50%" />
      <div class="skeleton-media-info">
        <SkeletonText lines={1} width="60%" />
      </div>
    </div>
  {/each}
</div>

<style>
  .skeleton-media-list {
    display: grid;
    gap: 0.75rem;
  }

  .skeleton-media-row {
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--bg-soft) 60%, var(--surface));
  }

  .skeleton-media-info {
    min-width: 0;
  }
</style>
