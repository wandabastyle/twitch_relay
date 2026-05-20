<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
  }

  const { children }: Props = $props();
</script>

<section class="twitch-panel">
  {@render children()}
</section>

<style>
  .twitch-panel {
    width: min(46rem, 100%);
    background: linear-gradient(
      160deg,
      color-mix(in srgb, var(--surface) 95%, transparent),
      color-mix(in srgb, var(--bg-soft) 95%, transparent)
    );
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }
</style>
