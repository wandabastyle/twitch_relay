<script lang="ts">
  import YouTubeSubscriptionsView from '$lib/components/YouTubeSubscriptionsView.svelte';
  import YouTubePlaylistsView from '$lib/components/YouTubePlaylistsView.svelte';
  import type { YouTubeModeViewProps } from './types';

  let { youtubeViewMode, onViewModeChange }: YouTubeModeViewProps = $props();
</script>

<div class="youtube-view">
  <div class="channels-header">
    <div class="channels-title-row youtube-tabs">
      <button
        type="button"
        class="channels-label tab"
        class:active={youtubeViewMode === 'subscriptions'}
        onclick={() => onViewModeChange('subscriptions')}
      >
        Subscribed Channels
      </button>
      <button
        type="button"
        class="channels-label tab"
        class:active={youtubeViewMode === 'playlists'}
        onclick={() => onViewModeChange('playlists')}
      >
        Playlists
      </button>
    </div>
  </div>
  {#if youtubeViewMode === 'subscriptions'}
    <YouTubeSubscriptionsView />
  {:else}
    <YouTubePlaylistsView />
  {/if}
</div>

<style>
  .youtube-view {
    display: grid;
    gap: 1rem;
  }

  .channels-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    margin-bottom: 0.75rem;
  }

  .channels-title-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .channels-label {
    font-weight: 600;
    color: var(--fg);
  }

  .youtube-tabs {
    gap: 1.25rem;
  }

  .youtube-tabs .tab {
    background: none;
    border: none;
    padding: 0.35rem 0.1rem;
    cursor: pointer;
    position: relative;
    color: var(--muted);
    transition: color 0.2s ease;
  }

  .youtube-tabs .tab:hover {
    color: var(--fg);
  }

  .youtube-tabs .tab.active {
    color: var(--fg);
  }

  .youtube-tabs .tab.active::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
  }

  @media (max-width: 600px) {
    .channels-title-row {
      flex-wrap: wrap;
    }

    .channels-header {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
