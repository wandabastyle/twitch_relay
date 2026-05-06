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

</script>

<div class="youtube-subscriptions">
  {#if isLoading}
    <p class="muted">Loading subscriptions...</p>
  {:else if error}
    <p class="error" role="alert">{error}</p>
  {:else if channels.length === 0}
    <p class="muted">No subscriptions found.</p>
  {:else}
    <div class="channels-list">
      {#each channels as channel (channel.channel_id)}
        <button
          type="button"
          class="channel-row"
          onclick={() => openChannel(channel.channel_id)}
        >
          <div class="channel-avatar-wrap">
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
          </div>
          <div class="channel-main">
            <span class="channel-name" title={channel.name}>{channel.name}</span>
            {#if channel.description}
              <span class="channel-description" title={channel.description}>{channel.description}</span>
            {/if}
          </div>
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

  .channels-list {
    display: grid;
    gap: 0.75rem;
  }

  .channel-row {
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--bg-soft) 60%, var(--surface));
    padding: 0.8rem;
    text-align: left;
    color: inherit;
    cursor: pointer;
    transition: border-color 0.2s ease, background-color 0.2s ease;
  }

  .channel-row:hover,
  .channel-row:focus-visible {
    border-color: var(--accent-border);
    background: var(--accent-soft);
    outline: none;
  }

  .channel-row:focus-visible {
    box-shadow: 0 0 0 3px var(--focus-ring);
  }

  .channel-avatar-wrap {
    height: 100%;
    min-height: 74px;
    display: flex;
    align-items: center;
  }

  .channel-avatar {
    width: 74px;
    height: 74px;
    border-radius: 50%;
    object-fit: cover;
    display: block;
    background: var(--surface-2);
  }

  .channel-avatar.fallback {
    display: grid;
    place-items: center;
    text-transform: uppercase;
    font-weight: 700;
    font-size: 1.35rem;
    color: var(--fg);
  }

  .channel-main {
    min-width: 0;
    display: grid;
    gap: 0.28rem;
  }

  .channel-name {
    font-size: 0.95rem;
    font-weight: 600;
    text-align: left;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .channel-description {
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1.4;
    display: -webkit-box;
    line-clamp: 3;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: pre-line;
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
