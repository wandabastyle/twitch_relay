<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { createYouTubeWatchPageController } from './you-tube-watch-page-controller.svelte';

  const HOURS_IN_SECONDS = 3600;
  const MINUTES_IN_SECONDS = 60;
  const PAD_LENGTH = 2;
  const ZERO = 0;

  interface Props {
    video_id: string;
  }

  const { video_id }: Props = $props();
  const playerFrameRef = $state<{ value: HTMLIFrameElement | null }>({ value: null });

  // Create controller reactively - recreated when video_id changes
  const controller = $derived(createYouTubeWatchPageController(video_id));

  $effect(() => {
    controller.setPlayerFrame(playerFrameRef.value);
  });

  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / HOURS_IN_SECONDS);
    const minutes = Math.floor((seconds % HOURS_IN_SECONDS) / MINUTES_IN_SECONDS);
    const secs = seconds % MINUTES_IN_SECONDS;

    if (hours > ZERO) {
      return `${hours}:${minutes.toString().padStart(PAD_LENGTH, '0')}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
    }
    return `${minutes}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
  }

  onMount(() => {
    controller.initialize();
  });

  onDestroy(() => {
    controller.stop();
  });
</script>

<section class="ui-page-panel ui-page-panel--wide">
  <header class="player-header">
    <div>
      <button type="button" class="ui-nav-chip" onclick={controller.goBack}>Back to videos</button>
      <h1>{controller.videoTitle}</h1>
      {#if controller.videoDuration !== null}
        <p class="subtle">Duration: {formatDuration(controller.videoDuration)}</p>
      {/if}
    </div>
  </header>

  {#if controller.error !== null}
    <div class="player-wrapper">
      <div class="player error-box">
        <p class="ui-error" role="alert">{controller.error}</p>
      </div>
    </div>
  {:else if controller.isLoading}
    <div class="player-wrapper">
      <div class="player loading-box">
        <p class="ui-muted">Loading video...</p>
      </div>
    </div>
  {:else if video_id !== '' && controller.embedUrl !== ''}
    <div class="player-wrapper">
      <iframe
        bind:this={playerFrameRef.value}
        class="player"
        src={controller.embedUrl}
        title="Invidious video player"
        allow="autoplay; fullscreen; encrypted-media; picture-in-picture"
        allowfullscreen
        loading="eager"
        referrerpolicy={controller.referrerPolicy}
      ></iframe>
    </div>
  {:else}
    <div class="player-wrapper">
      <div class="player error-box">
        <p class="ui-error" role="alert">Unable to initialize player.</p>
      </div>
    </div>
  {/if}
</section>

<style>
  .player-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .player-header .ui-nav-chip {
    margin-bottom: 0.5rem;
  }

  /* .nav-chip-btn styles now provided by app.css via .ui-nav-chip */

  h1 {
    margin: 0.2rem 0 0;
    font-size: clamp(1.2rem, 3vw, 1.8rem);
    line-height: 1.3;
  }

  .subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.84rem;
    overflow-wrap: anywhere;
  }

  .player-wrapper {
    width: 100%;
    aspect-ratio: 16 / 9;
    min-height: 16rem;
    max-height: min(74vh, 52rem);
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: #000;
    overflow: hidden;
  }

  .player {
    width: 100%;
    height: 100%;
    border: none;
    display: block;
  }

  .error-box {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: #000;
  }

  .loading-box {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: color-mix(in srgb, var(--bg-soft) 50%, #000);
  }

  /* 720p-class landscape TV browsers (e.g., Xbox Edge) */
  @media screen
    and (min-width: 100px)
    and (max-width: 1400px)
    and (min-height: 600px)
    and (max-height: 800px)
    and (orientation: landscape) {
    .player-wrapper {
      max-height: min(70vh, 600px);
    }
  }
</style>
