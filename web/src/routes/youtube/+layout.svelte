<script lang="ts">
  import { onMount } from 'svelte';
  import AppVersion from '$lib/components/AppVersion.svelte';

  let { children } = $props();

  onMount(() => {
    // Set YouTube theme on body
    document.body.dataset.theme = 'youtube';
  });
</script>

<svelte:head>
  <title>YouTube Relay</title>
</svelte:head>

<div class="youtube-app">
  <main class="youtube-main">
    {@render children()}
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

  :global(.youtube-app .app-version) {
    margin-top: auto;
    padding: 1rem;
  }
</style>
