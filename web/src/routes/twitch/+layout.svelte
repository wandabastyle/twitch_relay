<script lang="ts">
  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';
  import AppVersion from '$lib/components/AppVersion.svelte';

  let { children } = $props();
  let mainElement = $state<HTMLElement | null>(null);

  onMount(() => {
    // Set Twitch theme on body
    document.body.dataset.theme = 'twitch';
  });

  // Focus management: only on forward navigations, not back/forward
  afterNavigate((navigation) => {
    // Skip focus management for popstate (back/forward) to preserve scroll position
    if (navigation.type === 'popstate') return;

    // Focus the main container for keyboard navigation, but don't scroll
    if (mainElement) {
      mainElement.focus({ preventScroll: true });
    }
  });
</script>

<svelte:head>
  <title>Twitch Relay</title>
</svelte:head>

  <div class="twitch-app">
  <main
    bind:this={mainElement}
    class="twitch-main"
    tabindex="-1"
    aria-label="Twitch Relay main content"
  >
    {@render children()}
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
