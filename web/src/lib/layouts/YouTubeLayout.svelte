<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { afterNavigate } from '$lib/router/router.svelte';
  import AppVersion from '$lib/components/AppVersion.svelte';

  let { children } = $props<{ children?: import('svelte').Snippet }>();
  let mainElement = $state<HTMLElement | null>(null);

  onMount(() => {
    // Set YouTube theme on body
    document.body.dataset.theme = 'youtube';
  });

  onDestroy(() => {
    // Clean up theme on destroy
    delete document.body.dataset.theme;
  });

  // Focus management: only on forward navigations, not back/forward
  // This preserves scroll position when returning from video player
  afterNavigate((navigation) => {
    // Skip focus management for popstate (back/forward) to preserve scroll position
    if (navigation.type === 'popstate') return;

    // Focus the main container for keyboard navigation, but don't scroll
    if (mainElement) {
      mainElement.focus({ preventScroll: true });
    }
  });
</script>

<div class="youtube-app">
  <main
    bind:this={mainElement}
    class="youtube-main"
    tabindex="-1"
    aria-label="YouTube Relay main content"
  >
    {@render children?.()}
  </main>

  <AppVersion />
</div>

<style>
  .youtube-app {
    min-height: 100dvh;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    background: radial-gradient(
      circle at 20% -10%,
      color-mix(in srgb, var(--surface-2) 88%, black) 0%,
      var(--bg-soft) 45%,
      var(--bg) 100%
    );
    color: var(--fg);
  }

  .youtube-main {
    flex: 1;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .youtube-main:focus {
    outline: none;
  }

  :global(.youtube-app .app-version) {
    margin-top: auto;
    padding: 1rem;
  }
</style>
