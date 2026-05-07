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
    // Store context before navigating
    if (typeof window !== 'undefined') {
      sessionStorage.setItem('youtubeBackContext', 'subscriptions');
    }
    goto(`/youtube/channel/${encodeURIComponent(channelId)}`);
  }

</script>

<div class="youtube-subscriptions">
  {#if isLoading}
    <p class="ui-muted">Loading subscriptions...</p>
  {:else if error}
    <p class="ui-error" role="alert">{error}</p>
  {:else if channels.length === 0}
    <p class="ui-muted">No subscriptions found.</p>
  {:else}
    <div class="ui-list">
      {#each channels as channel (channel.channel_id)}
        <button
          type="button"
          class="ui-card ui-card-interactive channel-row"
          onclick={() => openChannel(channel.channel_id)}
        >
          <div class="ui-media-visual channel-avatar-wrap">
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
          </div>
          <div class="ui-media-main channel-main">
            <span class="ui-media-title channel-name" title={channel.name}>{channel.name}</span>
            {#if channel.description}
              <span class="ui-media-meta channel-description" title={channel.description}>{channel.description}</span>
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

  /* Component-specific layout and sizing for shared classes */
  .channel-row {
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
  }

  .channel-avatar-wrap {
    height: 100%;
    min-height: 74px;
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

  /* .muted, .error, .channels-list, .channel-row base styles, .channel-avatar-wrap base,
     .channel-avatar base (except sizing), .channel-avatar.fallback base,
     .channel-main, .channel-name now provided by app.css via .ui-* classes */
</style>
