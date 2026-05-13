<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import ArrowLeftRight from 'lucide-svelte/icons/arrow-left-right';
  import { getYouTubePlaylists, getYouTubePlaylistThumbnailUrl } from '$lib/api';
  import type { YoutubePlaylist } from '$lib/api';
  import { formatTimeAgo } from '$lib/youtube/format';
  import { LoadedFade, YouTubeMediaRow } from '$lib/components/youtube';

  let playlists = $state<YoutubePlaylist[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      playlists = await getYouTubePlaylists();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load playlists';
    } finally {
      isLoading = false;
    }
  });

  function handlePlaylistClick(playlistId: string) {
    goto(`/youtube/playlist/${encodeURIComponent(playlistId)}`);
  }

  function getThumbnailUrl(playlist: YoutubePlaylist): string {
    return getYouTubePlaylistThumbnailUrl(playlist.playlist_id);
  }

  function getPlaylistInitial(title: string): string {
    return title.slice(0, 1).toUpperCase();
  }
</script>

<svelte:head>
  <title>Playlists - YouTube Relay</title>
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
      <p class="header-subtle">Your playlists</p>
    </div>
  </header>

  <nav class="youtube-nav">
    <a href="/youtube" class="nav-link">Subscriptions</a>
    <a href="/youtube/recent" class="nav-link">Recent</a>
    <a href="/youtube/playlists" class="nav-link active">Playlists</a>
  </nav>

  {#if error}
    <p class="ui-error" role="alert">{error}</p>
  {:else if !isLoading && playlists.length === 0}
    <p class="ui-muted">No playlists found.</p>
  {:else if !isLoading}
    <LoadedFade loaded={true}>
      <div class="ui-list">
        {#each playlists as playlist (playlist.playlist_id)}
          {#snippet visual()}
            <div class="playlist-thumbnail-container">
              <img
                class="ui-thumbnail playlist-thumbnail"
                src={getThumbnailUrl(playlist)}
                alt={playlist.title}
                loading="lazy"
                onerror={(e) => {
                  const img = e.currentTarget as HTMLImageElement;
                  img.style.display = 'none';
                  const fallback = img.nextElementSibling as HTMLElement | null;
                  if (fallback) fallback.style.display = 'grid';
                }}
              />
              <div class="playlist-thumbnail-fallback" style="display: none;">
                {getPlaylistInitial(playlist.title)}
              </div>
            </div>
          {/snippet}

          {#snippet meta()}
            <span class="ui-media-meta playlist-meta">
              {playlist.video_count} videos · Updated {formatTimeAgo(playlist.updated)}
            </span>
          {/snippet}

          <YouTubeMediaRow
            title={playlist.title}
            onClick={() => handlePlaylistClick(playlist.playlist_id)}
            {visual}
            {meta}
            extraClass="youtube-playlist-row"
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

  .playlist-thumbnail-container {
    position: relative;
    width: 140px;
    height: 78px;
  }

  .playlist-thumbnail {
    width: 140px;
    height: 78px;
    border-radius: 0.5rem;
  }

  .playlist-thumbnail-fallback {
    position: absolute;
    top: 0;
    left: 0;
    width: 140px;
    height: 78px;
    border-radius: 0.5rem;
    display: none;
    place-items: center;
    text-transform: uppercase;
    font-weight: 700;
    font-size: 1.5rem;
    color: var(--fg);
    background: var(--surface-2);
  }

  :global(.youtube-playlist-row) {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
  }
</style>
