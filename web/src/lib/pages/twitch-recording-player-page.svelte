<script lang="ts">
  import { onDestroy } from 'svelte';
  import { navigate, page } from '$lib/router/router.svelte';
  import { createRecordingPlayerController } from './twitch-recording-player-page-controller.svelte';

  let isInitialized = $state(false);
  let playerElement = $state<HTMLVideoElement | null>(null);
  const controller = createRecordingPlayerController();

  $effect(() => {
    controller.setPlayerElement(playerElement);
  });

  const goBack = (): void => navigate('/twitch/recordings');

  $effect(() => {
    const { query } = page as unknown as { query?: Record<string, string> };
    controller.setParams(query?.channel_login ?? '', query?.filename ?? '');
  });

  $effect(() => {
    if (
      playerElement === null
      || controller.channelLogin === ''
      || controller.filename === ''
      || isInitialized
    ) {
      return;
    }

    isInitialized = true;
    void controller.initializePlayer();
  });

  onDestroy(() => {
    controller.teardown();
  });
</script>

<section class="ui-page-panel ui-page-panel--wide">
  <header class="ui-page-header">
    <div>
      <p class="ui-page-eyebrow">Recording Playback</p>
      <h1 class="ui-page-title">{controller.channelLogin || 'unknown channel'}</h1>
      {#if controller.filename}
        <p class="ui-page-subtle" title={controller.filename}>{controller.filename}</p>
      {/if}
    </div>
    <button type="button" class="ui-nav-chip" onclick={goBack}>Back to recordings</button>
  </header>

  {#if !controller.channelLogin || !controller.filename}
    <p class="ui-error">Missing recording playback parameters.</p>
  {:else}
    <div class="player-wrapper">
      {#if controller.isLoading}
        <div class="player loading">
          <p class="ui-muted">Loading player...</p>
        </div>
      {/if}
      <video class="player" class:hidden={controller.isLoading} controls preload="auto" bind:this={playerElement}>
        Your browser cannot play this recording format.
      </video>
    </div>
    {#if controller.playbackError}
      <p class="ui-error" role="alert">{controller.playbackError}</p>
    {/if}
  {/if}
</section>

<style>
  /* Player-specific styles - not shared */
  .ui-page-panel--wide {
    display: grid;
    gap: 0.8rem;
  }

  .player-wrapper {
    width: 100%;
    aspect-ratio: 16 / 9;
    min-height: 16rem;
    max-height: min(74vh, 52rem);
    border: 1px solid rgba(180, 198, 236, 0.35);
    background: #000;
    overflow: hidden;
  }

  .player {
    width: 100%;
    height: 100%;
    border-radius: 0;
    border: none;
    background: #000;
    display: block;
  }

  .player.loading {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .player.hidden {
    display: none;
  }

  /* 720p-class landscape TV browsers (e.g., Xbox Edge) */
  @media screen
    and (min-width: 1000px)
    and (max-width: 1400px)
    and (min-height: 600px)
    and (max-height: 800px)
    and (orientation: landscape) {
    .player-wrapper {
      max-height: min(70vh, 600px);
    }
  }
</style>
