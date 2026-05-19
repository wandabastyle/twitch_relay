<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '$lib/router/router.svelte';
  import { getYouTubeSubscriptions } from '$lib/api-client';
  import type { YoutubeChannel } from '$lib/api-client';
  import { LoadedFade, YouTubeMediaRow, YouTubeShell } from '$lib/components/youtube';
  import { SkeletonMediaList, ErrorState, EmptyState } from '$lib/components/ui';

  let channels = $state<YoutubeChannel[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  async function loadSubscriptions(): Promise<void> {
    isLoading = true;
    error = null;
    try {
      channels = await getYouTubeSubscriptions();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load subscriptions';
    } finally {
      isLoading = false;
    }
  }

  onMount(loadSubscriptions);

  function openChannel(channelId: string) {
    navigate(`/youtube/channel/${encodeURIComponent(channelId)}`);
  }
</script>

<YouTubeShell activeTab="subscriptions">
  {#if isLoading}
    <SkeletonMediaList count={8} />
  {:else if error}
    <ErrorState
      message={error}
      onRetry={loadSubscriptions}
      isRetrying={isLoading}
    />
  {:else if channels.length === 0}
    <EmptyState
      title="No subscriptions found"
      description="Subscribe to YouTube channels in Invidious to see them here."
      variant="channels"
    />
  {:else}
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
</YouTubeShell>

<style>
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
