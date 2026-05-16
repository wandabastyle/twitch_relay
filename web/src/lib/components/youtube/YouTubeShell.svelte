<script lang="ts">
  import { goto } from '$app/navigation';
  import RelayHeader from '$lib/components/shared/RelayHeader.svelte';

  interface Props {
    activeTab: 'subscriptions' | 'recent' | 'playlists';
    subtitle?: string;
    children?: import('svelte').Snippet;
  }

  let { activeTab, subtitle = 'Invidious subscriptions', children }: Props = $props();

  function switchToTwitch(): void {
    goto('/twitch');
  }
</script>

<section class="youtube-panel">
  <RelayHeader
    eyebrow="Private Deck"
    title="YouTube Relay"
    subtitleText={subtitle}
    onToggle={switchToTwitch}
    toggleLabel="Switch to Twitch Relay"
  />

  <nav class="youtube-nav">
    <a href="/youtube" class="nav-link" class:active={activeTab === 'subscriptions'}>Subscriptions</a>
    <a href="/youtube/recent" class="nav-link" class:active={activeTab === 'recent'}>Recent</a>
    <a href="/youtube/playlists" class="nav-link" class:active={activeTab === 'playlists'}>Playlists</a>
  </nav>

  {@render children?.()}
</section>

<style>
  .youtube-panel {
    width: min(46rem, 100%);
    background: linear-gradient(160deg, color-mix(in srgb, var(--surface) 95%, transparent), color-mix(in srgb, var(--bg-soft) 95%, transparent));
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

  .youtube-nav {
    display: flex;
    gap: 1.25rem;
    margin-bottom: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  }

  .nav-link {
    color: var(--muted);
    text-decoration: none;
    font-size: 0.9rem;
    font-weight: 500;
    padding: 0.35rem 0;
    border-bottom: 2px solid transparent;
    transition: color 0.2s ease, border-color 0.2s ease;
  }

  .nav-link:hover {
    color: var(--fg);
  }

  .nav-link.active {
    color: var(--fg);
    border-bottom-color: var(--accent);
  }
</style>
