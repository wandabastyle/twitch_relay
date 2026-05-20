<script lang="ts">
  import { onMount } from 'svelte';
  import {
    AUTO_LEVEL,
    ensureHlsLoaded,
    getHlsClass,
    HLS_PATH,
    type HlsInstance,
    qualityLabel,
    selectedQualityLabel,
    setQuality,
  } from '$lib/components/watch/video-player-utils';
  import {
    attachHlsEvents,
    setupHlsInstance,
  } from '$lib/components/watch/video-player-hls-setup';
  import {
    attachPlayerEvents,
    cleanupPlayer,
    createGoLive,
    createUpdateGoLiveState,
  } from '$lib/components/watch/video-player-events';

  interface Props {
    manifestUrl: string;
    onError: (message: string) => void;
  }

  const { manifestUrl, onError }: Props = $props();

  let currentPlayingLevel = $state(AUTO_LEVEL);
  let hlsInstance: HlsInstance | null = null;
  let hlsLevels = $state<
    {
      bitrate: number;
      height: number;
      name?: string;
    }[]
  >([]);
  let liveButtonIsLive = $state(true);
  let playerEl = $state<HTMLVideoElement | null>(null);
  let qualityLevel = $state(AUTO_LEVEL);
  let qualityMenuOpen = $state(false);
  let userSelectedAuto = $state(true);

  // Create reactive actions
  const setHlsLevels = (levels: typeof hlsLevels): void => {
    hlsLevels = levels;
  };
  const setCurrentPlayingLevel = (level: number): void => {
    currentPlayingLevel = level;
  };
  const setLiveButtonIsLive = (isLive: boolean): void => {
    liveButtonIsLive = isLive;
  };
  const setQualityLevel = (level: number): void => {
    qualityLevel = level;
  };
  const setUserSelectedAuto = (auto: boolean): void => {
    userSelectedAuto = auto;
  };

  const updateGoLiveState = createUpdateGoLiveState(
    () => playerEl,
    () => liveButtonIsLive,
    setLiveButtonIsLive,
  );

  const goLive = createGoLive(
    () => playerEl,
    () => liveButtonIsLive,
    () => hlsInstance,
    updateGoLiveState,
  );

  const handleQualityLevel = (level: number): void => {
    setQuality(level, hlsInstance);
    setQualityLevel(level);
    setUserSelectedAuto(level === AUTO_LEVEL);
  };

  const selectQuality = (level: number): void => {
    handleQualityLevel(level);
    qualityMenuOpen = false;
  };

  const toggleQualityMenu = (): void => {
    qualityMenuOpen = !qualityMenuOpen;
  };

  const handleDocumentClick = (event: MouseEvent): void => {
    const target = event.target as HTMLElement;
    const qualityMenu = document.querySelector('.watch-overlay-quality-menu');
    const qualityBtn = document.querySelector('.watch-overlay-btn.quality-btn');

    if (
      qualityMenuOpen
      && qualityMenu
      && qualityBtn
      && !qualityMenu.contains(target)
      && !qualityBtn.contains(target)
    ) {
      qualityMenuOpen = false;
    }
  };

  const setupHlsPlayback = (HlsClass: ReturnType<typeof getHlsClass>): void => {
    if (!HlsClass || !playerEl) {
      return;
    }
    const instance = setupHlsInstance(HlsClass);
    hlsInstance = instance;
    setQualityLevel(AUTO_LEVEL);
    setCurrentPlayingLevel(AUTO_LEVEL);
    setUserSelectedAuto(true);

    attachHlsEvents(instance, HlsClass, {
      onError,
      qualityLevel,
      setCurrentPlayingLevel,
      setHlsLevels,
      setQualityLevel,
      setUserSelectedAuto,
      userSelectedAuto,
    });
    instance.loadSource(manifestUrl);
    instance.attachMedia(playerEl);
  };

  const setupNativePlayback = (): void => {
    if (playerEl && playerEl.canPlayType('application/vnd.apple.mpegurl')) {
      playerEl.src = manifestUrl;
    } else {
      onError('Your browser does not support HLS playback.');
    }
  };

  const setupPlayerWithHls = (HlsClass: ReturnType<typeof getHlsClass>): void => {
    if (!playerEl) {
      return;
    }

    if (HlsClass && HlsClass.isSupported()) {
      setupHlsPlayback(HlsClass);
    } else {
      setupNativePlayback();
    }
  };

  const setupPlayer = async (): Promise<void> => {
    if (!playerEl) {
      return;
    }

    const hlsLoaded = await ensureHlsLoaded(HLS_PATH);
    if (!hlsLoaded) {
      onError('Failed to load HLS player.');
      return;
    }

    const HlsClass = getHlsClass();
    if (HlsClass) {
      setupPlayerWithHls(HlsClass);
    }

    attachPlayerEvents(playerEl, updateGoLiveState, onError);
  };

  const handleCleanup = (): void => {
    cleanupPlayer(playerEl, updateGoLiveState, hlsInstance);
  };

  onMount(() => {
    const setup = setupPlayer();
    setup.catch(() => {
      // Ignore setup errors as they're handled via onError callback
    });

    return handleCleanup;
  });
</script>

<svelte:document onclick={handleDocumentClick} />

<div class="video-shell">
  <video bind:this={playerEl} class="video" autoplay controls playsinline>
    Your browser cannot play this stream format.
  </video>

  <div class="overlay-controls">
    <div class="overlay-left">
      <button
        type="button"
        class="overlay-btn go-live-btn"
        class:live={liveButtonIsLive}
        disabled={liveButtonIsLive}
        onclick={goLive}
      >
        {#if liveButtonIsLive}
          Live
        {:else}
          Go Live
        {/if}
      </button>
    </div>
    <div class="overlay-right">
      <button type="button" class="overlay-btn quality-btn" onclick={toggleQualityMenu}>
        {selectedQualityLabel(qualityLevel, currentPlayingLevel, hlsLevels)}
      </button>
      <div class="quality-menu" class:open={qualityMenuOpen}>
        <button
          type="button"
          class="quality-item"
          class:active={qualityLevel === AUTO_LEVEL}
          onclick={() => selectQuality(AUTO_LEVEL)}
        >
          Auto
        </button>
        {#each hlsLevels as level, idx (idx)}
          <button
            type="button"
            class="quality-item"
            class:active={qualityLevel === idx}
            onclick={() => selectQuality(idx)}
          >
            {qualityLabel(level, idx, hlsLevels)}
          </button>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .video-shell {
    position: relative;
    flex: 1;
    min-height: 0;
    background: #000;
    border: 1px solid var(--border);
    width: 100%;
  }

  .video {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }

  .overlay-controls {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    background: linear-gradient(rgba(30, 32, 48, 0.92), transparent);
    padding: 8px 10px 20px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    z-index: 10;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s;
  }

  .video-shell:hover .overlay-controls,
  .overlay-controls:focus-within {
    opacity: 1;
    pointer-events: auto;
  }

  /* Always show controls on touch devices */
  @media (hover: none) {
    .overlay-controls {
      opacity: 1;
      pointer-events: auto;
    }
  }

  .overlay-left,
  .overlay-right {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .overlay-btn {
    background: color-mix(in srgb, var(--surface) 45%, transparent);
    border: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
    color: var(--fg);
    cursor: pointer;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 600;
    line-height: 1;
    transition:
      background 0.15s,
      border-color 0.15s;
  }

  .overlay-btn:hover {
    background: var(--surface);
    border-color: var(--accent);
  }

  .overlay-btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .go-live-btn {
    background: color-mix(in srgb, #ff757f 24%, transparent);
    border-color: color-mix(in srgb, #ff757f 56%, transparent);
  }

  .go-live-btn.live {
    background: color-mix(in srgb, #eb0400 40%, transparent);
    border-color: color-mix(in srgb, #eb0400 74%, transparent);
    color: white;
  }

  .quality-menu {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: rgba(34, 36, 54, 0.96);
    border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
    border-radius: 6px;
    padding: 6px 0;
    min-width: 140px;
    display: none;
    z-index: 20;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  }

  .quality-menu.open {
    display: block;
  }

  .quality-item {
    width: 100%;
    padding: 8px 14px;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    justify-content: space-between;
    background: transparent;
    border: 0;
    color: var(--fg);
    text-align: left;
  }

  .quality-item:hover {
    background: color-mix(in srgb, var(--surface) 75%, transparent);
  }

  .quality-item.active {
    color: var(--accent-2);
    font-weight: 600;
  }
</style>
