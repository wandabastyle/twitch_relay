<script lang="ts">
  import { onMount } from 'svelte';

  import { getYouTubeSubscriptions, type YoutubeChannel } from '$lib/api-client';
  import { EmptyState, ErrorState, SkeletonMediaList } from '$lib/components/ui';
  import { LoadedFade, YouTubeMediaRow, YouTubeShell } from '$lib/components/youtube';
  import { navigate } from '$lib/router/router.svelte';

  const FAILED_TO_LOAD = 'Failed to load subscriptions';
  const LIST_ITEM_COUNT = 8;
  const NO_SUBS_DESC = 'Subscribe to YouTube channels in Invidious to see them here.';
  const NO_SUBS_TITLE = 'No subscriptions found';

  const INITIAL_SLICE_INDEX = 0;
  const INITIAL_SLICE_LENGTH = 1;

  let channels = $state<YoutubeChannel[]>([]);
  let isLoading = $state(true);
  let error = $state<string | undefined>(undefined);

  const loadSubscriptions = async (): Promise<void> => {
    isLoading = true;
    error = undefined;
    try {
      channels = await getYouTubeSubscriptions();
    } catch (error_) {
      const errorMessage = error_ instanceof Error ? error_.message : FAILED_TO_LOAD;
      error = errorMessage;
    } finally {
      isLoading = false;
    }
  };

  onMount(loadSubscriptions);

  const openChannel = (channelId: string): void => {
    navigate(`/youtube/channel/${encodeURIComponent(channelId)}`);
  };
</script>

<YouTubeShell activeTab="subscriptions">
  {#if isLoading}
    <SkeletonMediaList count={LIST_ITEM_COUNT} />
  {:else if error}
    <ErrorState
      message={error}
      onRetry={loadSubscriptions}
      isRetrying={isLoading}
    />
  {:else if channels.length === 0}
    <EmptyState
      title={NO_SUBS_TITLE}
      description={NO_SUBS_DESC}
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
                {channel.name.slice(INITIAL_SLICE_INDEX, INITIAL_SLICE_LENGTH)}
              </div>
            {/if}
          {/snippet}

          {#snippet meta()}
            {#if channel.description}
              <span class="ui-media-meta channel-description" title={channel.description}>{channel.description}</span>
            {/if}
          {/snippet}

          {@const metaSnippet = channel.description ? meta : undefined}
          <YouTubeMediaRow
            title={channel.name}
            onClick={() => openChannel(channel.channel_id)}
            {visual}
            meta={metaSnippet}
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
