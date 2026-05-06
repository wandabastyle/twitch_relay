<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getYouTubeSubscriptions } from '$lib/api';
  import type { YoutubeChannel } from '$lib/api';

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

  function formatSubscriberCount(count: number): string {
    if (count >= 1000000) {
      return `${(count / 1000000).toFixed(1)}M`;
    }
    if (count >= 1000) {
      return `${(count / 1000).toFixed(1)}K`;
    }
    return String(count);
  }
</script>

<div class="youtube-subscriptions">
  {#if isLoading}
    <p class="muted">Loading subscriptions...</p>
  {:else if error}
    <p class="error" role="alert">{error}</p>
  {:else if channels.length === 0}
    <p class="muted">No subscriptions found.</p>
  {:else}
    <div class="channels-grid">
      {#each channels as channel (channel.channel_id)}
        <button
          type="button"
          class="channel-card"
          onclick={() => openChannel(channel.channel_id)}
        >
          {#if channel.avatar}
            <img
              class="channel-avatar"
              src={channel.avatar}
              alt={channel.name}
              loading="lazy"
            />
          {:else}
            <div class="channel-avatar fallback">
              {channel.name.slice(0, 1)}
            </div>
          {/if}
          <span class="channel-name" title={channel.name}>{channel.name}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .youtube-subscriptions {
    display: grid;
    gap: 1rem;
  }

  .channels-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 1rem;
  }

  .channel-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    background: var(--surface);
    cursor: pointer;
    transition: border-color 0.2s ease, background-color 0.2s ease;
  }

  .channel-card:hover {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--surface) 90%, var(--accent));
  }

  .channel-avatar {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    object-fit: cover;
    background: var(--surface-2);
  }

  .channel-avatar.fallback {
    display: grid;
    place-items: center;
    text-transform: uppercase;
    font-weight: 700;
    font-size: 1.5rem;
    color: var(--fg);
  }

  .channel-name {
    font-size: 0.9rem;
    font-weight: 600;
    text-align: center;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
  }

  .muted {
    margin: 0;
    color: var(--muted);
  }

  .error {
    margin: 0;
    padding: 0.75rem;
    background: rgba(255, 82, 82, 0.15);
    border: 1px solid var(--danger);
    border-radius: 0.5rem;
    color: var(--danger);
  }
</style>
