<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import AppVersion from '$lib/components/AppVersion.svelte';

  let { children } = $props();

  onMount(() => {
    // Set YouTube theme on body
    document.body.dataset.theme = 'youtube';
  });

  function goToTwitch() {
    goto('/twitch');
  }
</script>

<svelte:head>
  <title>YouTube Relay</title>
</svelte:head>

<div class="youtube-app">
  <header class="mode-header">
    <div class="mode-title">
      <h1>YouTube Relay</h1>
      <button type="button" class="mode-switch" onclick={goToTwitch}>
        Switch to Twitch
      </button>
    </div>
  </header>
  
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

  .mode-header {
    padding: 1rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  }

  .mode-title {
    width: min(74rem, 96vw);
    margin: 0 auto;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .mode-title h1 {
    margin: 0;
    font-size: clamp(1.2rem, 3vw, 1.6rem);
    color: var(--fg);
  }

  .mode-switch {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
    border-radius: 0.6rem;
    color: var(--muted);
    padding: 0.4rem 0.8rem;
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
    transition: border-color 0.2s ease, color 0.2s ease;
  }

  .mode-switch:hover {
    border-color: var(--accent-border);
    color: var(--fg);
  }

  .youtube-main {
    flex: 1;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }

  .youtube-main::-webkit-scrollbar {
    display: none;
  }

  :global(.youtube-app .app-version) {
    margin-top: auto;
    padding: 1rem;
  }
</style>
