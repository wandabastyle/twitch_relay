<script lang="ts">
  import type { Snippet } from 'svelte';

  const DEFAULT_DURATION_MS = 280;

  const {
    loaded = true,
    duration = DEFAULT_DURATION_MS,
    children
  }: {
    loaded?: boolean;
    duration?: number;
    children?: Snippet;
  } = $props();
</script>

<div
  class="loaded-fade"
  class:loaded
  style={`--loaded-fade-duration: ${duration}ms`}
>
  {@render children?.()}
</div>

<style>
  .loaded-fade {
    opacity: 0;
  }

  .loaded-fade.loaded {
    animation: loadedFadeIn var(--loaded-fade-duration, 180ms) ease both;
  }

  @keyframes loadedFadeIn {
    from {
      opacity: 0;
    }

    to {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .loaded-fade {
      opacity: 1;
    }

    .loaded-fade.loaded {
      animation: none;
    }
  }
</style>
