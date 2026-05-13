<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getYouTubeSubscriptions } from '$lib/api';
  import type { YoutubeChannel } from '$lib/api';
  import { LoadedFade, YouTubeListState, YouTubeMediaRow } from '$lib/components/youtube';

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
</div>

<style>
  .youtube-subscriptions {
    display: grid;
    gap: 1rem;
  }

  /* Component-specific layout and sizing for shared classes */
  :global(.youtube-channel-row) {
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr);
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem;
    text-align: left;
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
</style>
