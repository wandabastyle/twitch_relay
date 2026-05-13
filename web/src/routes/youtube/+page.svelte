<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
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
  <header class="youtube-header">
    <h1 class="youtube-title">Subscribed Channels</h1>
    <nav class="youtube-nav">
      <a href="/youtube" class="nav-link active">Subscriptions</a>
      <a href="/youtube/recent" class="nav-link">Recent</a>
      <a href="/youtube/playlists" class="nav-link">Playlists</a>
    </nav>
  </header>

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

  .youtube-header {
    margin-bottom: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  }

  .youtube-title {
    margin: 0 0 0.75rem;
    font-size: clamp(1.2rem, 3vw, 1.6rem);
    color: var(--fg);
  }

  .youtube-nav {
    display: flex;
    gap: 1.25rem;
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
