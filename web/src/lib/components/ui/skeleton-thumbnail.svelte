<script lang="ts">
  interface Props {
    aspectRatio?: string;
    borderRadius?: string;
    height?: string;
    width?: string;
  }

  const {
    aspectRatio = "16 / 9",
    borderRadius = "0.5rem",
    height = "auto",
    width = "100%",
  }: Props = $props();
</script>

<div
  class="skeleton-thumbnail"
  style="width: {width}; height: {height}; aspect-ratio: {aspectRatio}; border-radius: {borderRadius}"
></div>

<style>
  .skeleton-thumbnail {
    max-width: 100%;
    background: linear-gradient(
      90deg,
      var(--surface-2) 25%,
      color-mix(in srgb, var(--surface-2) 70%, var(--surface)) 50%,
      var(--surface-2) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-pulse 1.5s ease-in-out infinite;
    flex-shrink: 0;
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
    .skeleton-thumbnail {
      animation: none;
      background: var(--surface-2);
    }
  }
</style>
