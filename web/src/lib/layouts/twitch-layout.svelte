<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import AppVersion from '$lib/components/app-version.svelte';
  import { afterNavigate } from '$lib/router/router.svelte';

  const POPSTATE_TYPE = 'popstate';

  const { children } = $props<{ children?: import('svelte').Snippet }>();
  // eslint-disable-next-line init-declarations -- Svelte bind:this requires let
  // eslint-disable-next-line prefer-const -- Svelte bind:this mutates the variable
  let mainElement = $state<HTMLElement>();

  onMount(() => {
    // Set Twitch theme on body
    document.body.dataset.theme = 'twitch';
  });

  onDestroy(() => {
    // Clean up theme on destroy
    delete document.body.dataset.theme;
  });

  // Focus management: only on forward navigations, not back/forward
  afterNavigate((navigation) => {
    // Skip focus management for popstate (back/forward) to preserve scroll position
    if (navigation.type === POPSTATE_TYPE) {
      return;
    }

    // Focus the main container for keyboard navigation, but don't scroll
    if (mainElement) {
      mainElement.focus({ preventScroll: true });
    }
  });
</script>

<div class="twitch-app">
  <main
    bind:this={mainElement}
    class="twitch-main"
    tabindex="-1"
    aria-label="Twitch Relay main content"
  >
    {@render children?.()}
  </main>

  <AppVersion />
</div>

<style>
  .twitch-app {
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

  .twitch-main {
    flex: 1;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .twitch-main:focus {
    outline: none;
  }

  :global(.twitch-app .app-version) {
    margin-top: auto;
    padding: 1rem;
  }
</style>
