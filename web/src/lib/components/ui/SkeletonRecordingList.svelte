<script lang="ts">
  import SkeletonText from './SkeletonText.svelte';

  interface Props {
    sections?: number;
    itemsPerSection?: number;
  }

  let { sections = 3, itemsPerSection = 3 }: Props = $props();
</script>

<div class="skeleton-recordings">
  {#each Array(sections) as _, sectionIndex (sectionIndex)}
    <div class="skeleton-section">
      <div class="skeleton-section-header">
        <SkeletonText lines={1} width="120px" />
      </div>
      <div class="skeleton-list">
        {#each Array(itemsPerSection) as _, itemIndex (`${sectionIndex}-${itemIndex}`)}
          <div class="skeleton-item">
            <div class="skeleton-item-content">
              <SkeletonText lines={1} width="60%" />
              <SkeletonText lines={1} width="40%" />
            </div>
            <div class="skeleton-item-actions">
              <div class="skeleton-action"></div>
              <div class="skeleton-action"></div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/each}
</div>

<style>
  .skeleton-recordings {
    display: grid;
    gap: 0.75rem;
  }

  .skeleton-section {
    border: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
    background: color-mix(in srgb, var(--bg-soft) 62%, #0a101b);
    border-radius: 0.75rem;
    padding: 0.8rem;
  }

  .skeleton-section-header {
    margin-bottom: 0.55rem;
  }

  .skeleton-list {
    display: grid;
    gap: 0.45rem;
  }

  .skeleton-item {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0;
  }

  .skeleton-item-content {
    display: grid;
    gap: 0.25rem;
    min-width: 0;
  }

  .skeleton-item-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  .skeleton-action {
    width: 2rem;
    height: 2rem;
    border-radius: 0.55rem;
    background: linear-gradient(
      90deg,
      var(--surface-2) 25%,
      color-mix(in srgb, var(--surface-2) 70%, var(--surface)) 50%,
      var(--surface-2) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-pulse 1.5s ease-in-out infinite;
  }

  @keyframes skeleton-pulse {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .skeleton-action {
      animation: none;
    }
  }
</style>
