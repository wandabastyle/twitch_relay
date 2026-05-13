<script lang="ts">
  interface Props {
    lines?: number;
    width?: string;
  }

  let { lines = 1, width = "100%" }: Props = $props();
</script>

<div class="skeleton-text" style="--lines: {lines}; --width: {width}">
  {#each Array(lines) as _, i}
    <div class="skeleton-line" style="width: {i === lines - 1 ? width : '100%'}"></div>
  {/each}
</div>

<style>
  .skeleton-text {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    width: 100%;
  }

  .skeleton-line {
    height: 0.9rem;
    background: linear-gradient(
      90deg,
      var(--surface-2) 25%,
      color-mix(in srgb, var(--surface-2) 70%, var(--surface)) 50%,
      var(--surface-2) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-pulse 1.5s ease-in-out infinite;
    border-radius: 0.25rem;
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
    .skeleton-line {
      animation: none;
      background: var(--surface-2);
    }
  }
</style>
