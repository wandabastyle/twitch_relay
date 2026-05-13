<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import ArrowLeftRight from 'lucide-svelte/icons/arrow-left-right';
  import { getYouTubeSubscriptions } from '$lib/api';
  import type { YoutubeChannel } from '$lib/api';
  import { LoadedFade, YouTubeMediaRow } from '$lib/components/youtube';

  let channels = $state<YoutubeChannel[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      channels = await getYouTubeSubscriptions();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load subscriptions';
    } finally {
      isLoading = false;
    }
  });

  function openChannel(channelId: string) {
    goto(`/youtube/channel/${encodeURIComponent(channelId)}`);
  }
</script>

<svelte:head>
  <title>Subscriptions - YouTube Relay</title>
</svelte:head>

<section class="youtube-panel">
  <header class="panel-header">
    <div class="panel-title">
      <p class="eyebrow">Private Deck</p>
      <button
        type="button"
        class="relay-title-button"
        onclick={() => goto('/twitch')}
        aria-label="Switch to Twitch Relay"
        title="Switch to Twitch Relay"
      >
        <h1>YouTube Relay</h1>
        <span class="toggle-icon" aria-hidden="true">
          <ArrowLeftRight size={14} />
        </span>
      </button>
      <p class="header-subtle">Invidious subscriptions</p>
    </div>
  </header>

  <nav class="youtube-nav">
    <a href="/youtube" class="nav-link active">Subscriptions</a>
    <a href="/youtube/recent" class="nav-link">Recent</a>
    <a href="/youtube/playlists" class="nav-link">Playlists</a>
  </nav>

  {#if error}
    <p class="ui-error" role="alert">{error}</p>
  {:else if !isLoading && channels.length === 0}
    <p class="ui-muted">No subscriptions found.</p>
  {:else if !isLoading}
    <LoadedFade loaded={true}>
      <div class="ui-list">
        {#each channels as channel (channel.channel_id)}
          {#snippet visual()}
            {#if channel.avatar}
              <img
                class="ui-avatar channel-avatar"
                src={channel.avatar}
                alt={channel.name}
                loading="lazy"
              />
            {:else}
              <div class="ui-avatar ui-avatar-fallback channel-avatar fallback">
                {channel.name.slice(0, 1)}
              </div>
            {/if}
          {/snippet}

          {#snippet meta()}
            {#if channel.description}
              <span class="ui-media-meta channel-description" title={channel.description}>{channel.description}</span>
            {/if}
          {/snippet}

          <YouTubeMediaRow
            title={channel.name}
            onClick={() => openChannel(channel.channel_id)}
            {visual}
            meta={channel.description ? meta : undefined}
            extraClass="youtube-channel-row"
          />
        {/each}
      </div>
    </LoadedFade>
  {/if}
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

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    position: relative;
  }

  .panel-title {
    min-width: 0;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .relay-title-button,
  .relay-title-button:hover,
  .relay-title-button:focus,
  .relay-title-button:active {
    text-decoration: none;
  }

  .relay-title-button {
    appearance: none;
    background: transparent;
    border: 0;
    padding: 0;
    margin: 0.2rem 0 0;
    font: inherit;
    font-weight: inherit;
    cursor: pointer;
    text-align: left;
    color: inherit;
    display: inline-flex;
    align-items: baseline;
    gap: 0.4rem;
  }

  .relay-title-button h1 {
    margin: 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  .toggle-icon {
    display: inline-flex;
    align-items: center;
    opacity: 0.45;
    transition: opacity 0.15s ease, transform 0.15s ease;
    color: var(--muted);
  }

  .relay-title-button:hover .toggle-icon {
    opacity: 0.9;
    color: var(--accent);
    transform: rotate(180deg);
  }

  .relay-title-button:hover {
    color: var(--accent);
  }

  .header-subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
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

  .channel-avatar {
    width: 74px;
    height: 74px;
    border-radius: 50%;
    font-size: 1.35rem;
  }

  .channel-description {
    line-height: 1.4;
    display: -webkit-box;
    line-clamp: 3;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: pre-line;
  }

  :global(.youtube-channel-row) {
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
  }
</style>
