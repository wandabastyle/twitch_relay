<script lang="ts">
  import { onMount } from 'svelte';

  // Hls.js types
  interface HlsInstance {
    currentLevel: number;
    destroy: () => void;
    loadSource: (url: string) => void;
    attachMedia: (element: HTMLVideoElement) => void;
    liveSyncPosition: number | null;
    on: (event: string, callback: (event: string, data: unknown) => void) => void;
  }

  interface HlsStatic {
    new (config: Record<string, unknown>): HlsInstance;
    isSupported: () => boolean;
    Events: {
      MANIFEST_PARSED: string;
      LEVEL_SWITCHED: string;
      ERROR: string;
    };
  }

  interface HlsLevel {
    bitrate: number;
    height: number;
    name?: string;
  }

  interface Props {
    manifestUrl: string;
    onError: (message: string) => void;
  }

  // Constants for magic numbers
  const AUTO_LEVEL = -1;
  const DEFAULT_LEVEL = -1;
  const SEEKABLE_INDEX_OFFSET = 1;
  const ZERO = 0;
  const ONE = 1;
  const MIN_SEEKABLE_LENGTH = 0;
  const SOURCE_BITRATE_DIVISOR = 1_000_000;
  const SOURCE_BITRATE_DECIMALS = 1;
  const SOURCE_HEIGHT_THRESHOLD = 1080;
  const SOURCE_INDEX = 0;
  const FIRST_LEVEL = 1;
  const HLS_LOAD_ATTEMPTS = 50;
  const HLS_LOAD_INTERVAL_MS = 100;
  const HLS_POLL_INCREMENT = 1;
  const HLS_PATH = '/static/hls.js';
  const LIVE_SYNC_START_POS = -6;
  const LIVE_SYNC_DURATION = 6;
  const LIVE_MAX_LATENCY = 14;
  const MAX_BUFFER_LENGTH = 20;
  const MAX_MAX_BUFFER_LENGTH = 45;
  const BACK_BUFFER_LENGTH = 15;
  const FRAG_LOADING_TIMEOUT = 20_000;
  const LEVEL_LOADING_TIMEOUT = 15_000;
  const MANIFEST_LOADING_TIMEOUT = 15_000;
  const FRAG_LOADING_MAX_RETRY = 5;
  const LEVEL_LOADING_MAX_RETRY = 3;
  const MANIFEST_LOADING_MAX_RETRY = 3;
  const RETRY_DELAY_MS = 750;
  const ABR_EWMA_FAST_LIVE = 3;
  const ABR_EWMA_SLOW_LIVE = 9;
  const MAX_LIVE_SYNC_PLAYBACK_RATE = 1.1;

  const { manifestUrl, onError }: Props = $props();

  let currentPlayingLevel = $state(AUTO_LEVEL);
  let hlsInstance: HlsInstance | null = null;
  let hlsLevels = $state<HlsLevel[]>([]);
  let liveButtonIsLive = $state(true);
  // eslint-disable-next-line init-declarations -- Svelte bind:this requires let
  let playerEl: HTMLVideoElement | null = null;
  let qualityLevel = $state(AUTO_LEVEL);
  let qualityMenuOpen = $state(false);
  let userSelectedAuto = $state(true);

  const RESUME_ENTER_LIVE_SECS = 5.5;
  const RESUME_EXIT_LIVE_SECS = 7.5;

  // Helper functions declared first to avoid use-before-define
  const toObject = (value: unknown): Record<string, unknown> | null => {
    if (typeof value === 'object' && value !== null) {
      return value as Record<string, unknown>;
    }
    return null;
  };

  const waitForHls = (): Promise<void> => {
    let attempts = ZERO;
    return new Promise((resolve) => {
      const check = (): void => {
        if (
          typeof globalThis !== 'undefined' &&
          !('Hls' in globalThis) &&
          attempts < HLS_LOAD_ATTEMPTS
        ) {
          setTimeout(() => {
            attempts += HLS_POLL_INCREMENT;
            check();
          }, HLS_LOAD_INTERVAL_MS);
        } else {
          resolve();
        }
      };
      check();
    });
  };

  const loadScript = (path: string): Promise<boolean> => {
    const script = document.createElement('script');
    script.src = path;
    script.async = true;

    return new Promise<boolean>((resolve) => {
      script.addEventListener('load', () => resolve(true));
      script.addEventListener('error', () => resolve(false));
      document.head.append(script);
    });
  };

  const ensureHlsLoaded = (path: string): Promise<boolean> => {
    if (typeof globalThis === 'undefined') {
      return Promise.resolve(false);
    }
    if ('Hls' in globalThis) {
      return Promise.resolve(true);
    }

    const existing = document.querySelector<HTMLScriptElement>(`script[src="${path}"]`);
    if (existing) {
      return waitForHls().then(() => 'Hls' in globalThis);
    }

    return loadScript(path).then((loaded) => {
      if (!loaded) {
        return false;
      }
      return waitForHls().then(() => 'Hls' in globalThis);
    });
  };

  const updateGoLiveState = (): void => {
    if (!playerEl || playerEl.seekable.length <= MIN_SEEKABLE_LENGTH) {
      liveButtonIsLive = true;
      return;
    }

    const end = playerEl.seekable.end(playerEl.seekable.length - SEEKABLE_INDEX_OFFSET);
    const lag = Math.max(ZERO, end - playerEl.currentTime);

    if (liveButtonIsLive) {
      if (lag > RESUME_EXIT_LIVE_SECS) {
        liveButtonIsLive = false;
      }
    } else if (lag < RESUME_ENTER_LIVE_SECS) {
      liveButtonIsLive = true;
    }
  };

  const cleanupPlayer = (): void => {
    if (playerEl) {
      playerEl.removeEventListener('timeupdate', updateGoLiveState);
      playerEl.removeEventListener('loadedmetadata', updateGoLiveState);
      playerEl.removeEventListener('durationchange', updateGoLiveState);
    }

    if (hlsInstance) {
      hlsInstance.destroy();
      hlsInstance = null;
    }
  };

  const goLive = (): void => {
    if (!playerEl || liveButtonIsLive) {
      return;
    }

    if (hlsInstance && Number.isFinite(hlsInstance.liveSyncPosition)) {
      playerEl.currentTime = hlsInstance.liveSyncPosition as number;
    } else if (playerEl.seekable.length > MIN_SEEKABLE_LENGTH) {
      playerEl.currentTime = playerEl.seekable.end(playerEl.seekable.length - SEEKABLE_INDEX_OFFSET);
    }
    updateGoLiveState();
  };

  const formatBitrate = (bitrate: number): string => {
    if (bitrate <= ZERO) {
      return '';
    }
    return ` (${(bitrate / SOURCE_BITRATE_DIVISOR).toFixed(SOURCE_BITRATE_DECIMALS)} Mbps)`;
  };

  const isSourceQuality = (level: HlsLevel, idx: number): boolean => {
    const isFirstLevel = idx === SOURCE_INDEX;
    const hasMultipleLevels = hlsLevels.length > FIRST_LEVEL;
    const isHighQuality = level.height >= SOURCE_HEIGHT_THRESHOLD;
    return level.name === 'Source' || (isFirstLevel && hasMultipleLevels && isHighQuality);
  };

  const qualityLabel = (level: HlsLevel, idx: number): string => {
    const source = isSourceQuality(level, idx);

    if (source) {
      return `Source${formatBitrate(level.bitrate)}`;
    }
    return `${level.height}p${formatBitrate(level.bitrate)}`;
  };

  const getQualityDisplay = (idx: number): string | null => {
    const level = hlsLevels[idx];
    if (!level) {
      return null;
    }

    const hasMultipleLevels = hlsLevels.length > FIRST_LEVEL;
    const isHighQuality = level.height >= SOURCE_HEIGHT_THRESHOLD;
    const isFirstLevel = idx === SOURCE_INDEX;
    const isSource = level.name === 'Source' || (isFirstLevel && hasMultipleLevels && isHighQuality);
    
    if (isSource) {
      return 'Source';
    }
    return `${level.height}p`;
  };

  const selectedQualityLabel = (): string => {
    if (qualityLevel === AUTO_LEVEL) {
      if (currentPlayingLevel >= ZERO && hlsLevels[currentPlayingLevel]) {
        return `Auto (${hlsLevels[currentPlayingLevel].height}p)`;
      }
      return 'Auto';
    }

    const display = getQualityDisplay(qualityLevel);
    
    if (display === null) {
      return 'Manual';
    }
    return display;
  };

  const setQuality = (level: number): void => {
    if (!hlsInstance) {
      return;
    }
    hlsInstance.currentLevel = level;
    qualityLevel = level;
    userSelectedAuto = level === AUTO_LEVEL;
  };

  const selectQuality = (level: number): void => {
    setQuality(level);
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
      qualityMenuOpen &&
      qualityMenu &&
      qualityBtn &&
      !qualityMenu.contains(target) &&
      !qualityBtn.contains(target)
    ) {
      qualityMenuOpen = false;
    }
  };

  // Event handlers for HLS
  const handleManifestParsed = (_event: string, data: unknown): void => {
    const parsed = toObject(data);
    if (!parsed || !Array.isArray(parsed.levels)) {
      hlsLevels = [];
      return;
    }
    
    hlsLevels = parsed.levels.filter((item): item is HlsLevel => {
      const obj = toObject(item);
      return obj !== null && typeof obj.height === 'number' && typeof obj.bitrate === 'number';
    });
  };

  const handleLevelSwitched = (_event: string, data: unknown): void => {
    const parsed = toObject(data);
    const { level: parsedLevel } = parsed ?? {};
    const level = typeof parsedLevel === 'number' ? parsedLevel : DEFAULT_LEVEL;
    
    currentPlayingLevel = level;
    if (userSelectedAuto) {
      qualityLevel = AUTO_LEVEL;
    }
  };

  const handleHlsError = (_event: string, data: unknown): void => {
    const parsed = toObject(data);
    if (parsed && parsed.fatal === true) {
      onError('Stream unavailable. The channel may be offline or not accessible.');
    }
  };

  const setupHlsInstance = (HlsClass: HlsStatic): HlsInstance => {
    const instance = new HlsClass({
      abrEwmaFastLive: ABR_EWMA_FAST_LIVE,
      abrEwmaSlowLive: ABR_EWMA_SLOW_LIVE,
      backBufferLength: BACK_BUFFER_LENGTH,
      capLevelToPlayerSize: true,
      fragLoadingMaxRetry: FRAG_LOADING_MAX_RETRY,
      fragLoadingRetryDelay: RETRY_DELAY_MS,
      fragLoadingTimeOut: FRAG_LOADING_TIMEOUT,
      levelLoadingMaxRetry: LEVEL_LOADING_MAX_RETRY,
      levelLoadingRetryDelay: RETRY_DELAY_MS,
      levelLoadingTimeOut: LEVEL_LOADING_TIMEOUT,
      liveMaxLatencyDuration: LIVE_MAX_LATENCY,
      liveSyncDuration: LIVE_SYNC_DURATION,
      lowLatencyMode: true,
      manifestLoadingMaxRetry: MANIFEST_LOADING_MAX_RETRY,
      manifestLoadingRetryDelay: RETRY_DELAY_MS,
      manifestLoadingTimeOut: MANIFEST_LOADING_TIMEOUT,
      maxBufferLength: MAX_BUFFER_LENGTH,
      maxLiveSyncPlaybackRate: MAX_LIVE_SYNC_PLAYBACK_RATE,
      maxMaxBufferLength: MAX_MAX_BUFFER_LENGTH,
      startLevel: AUTO_LEVEL,
      startPosition: LIVE_SYNC_START_POS,
    });

    instance.currentLevel = AUTO_LEVEL;
    return instance;
  };

  const attachHlsEvents = (instance: HlsInstance, HlsClass: HlsStatic): void => {
    instance.on(HlsClass.Events.MANIFEST_PARSED, handleManifestParsed);
    instance.on(HlsClass.Events.LEVEL_SWITCHED, handleLevelSwitched);
    instance.on(HlsClass.Events.ERROR, handleHlsError);
  };

  const attachPlayerEvents = (): void => {
    if (!playerEl) {
      return;
    }
    
    playerEl.addEventListener('timeupdate', updateGoLiveState);
    playerEl.addEventListener('loadedmetadata', updateGoLiveState);
    playerEl.addEventListener('durationchange', updateGoLiveState);
    playerEl.addEventListener('error', () => {
      onError('Stream unavailable. The channel may be offline or not accessible.');
    });
  };

  const setupPlayerWithHls = (HlsClass: HlsStatic): void => {
    if (!playerEl) {
      return;
    }
    
    if (HlsClass.isSupported()) {
      const instance = setupHlsInstance(HlsClass);
      hlsInstance = instance;
      qualityLevel = AUTO_LEVEL;
      currentPlayingLevel = AUTO_LEVEL;
      userSelectedAuto = true;

      attachHlsEvents(instance, HlsClass);
      instance.loadSource(manifestUrl);
      instance.attachMedia(playerEl);
    } else if (playerEl.canPlayType('application/vnd.apple.mpegurl')) {
      playerEl.src = manifestUrl;
    } else {
      onError('Your browser does not support HLS playback.');
    }
  };

  const setupPlayer = (): Promise<void> => {
    if (!playerEl) {
      return Promise.resolve();
    }

    return ensureHlsLoaded(HLS_PATH).then((hlsLoaded) => {
      if (!hlsLoaded) {
        onError('Failed to load HLS player.');
        return;
      }

      if ('Hls' in globalThis) {
        const HlsClass = (globalThis as unknown as { Hls: HlsStatic }).Hls;
        setupPlayerWithHls(HlsClass);
      }

      attachPlayerEvents();
    });
  };

  onMount(() => {
    // Explicitly handle promise
    const setup = setupPlayer();
    // Prevent unhandled rejection by attaching a no-op catch
    setup.catch(() => {
      // Ignore setup errors as they're handled via onError callback
    });
    
    return (): void => {
      cleanupPlayer();
    };
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
      <button
        type="button"
        class="overlay-btn quality-btn"
        onclick={toggleQualityMenu}
      >
        {selectedQualityLabel()}
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
            {qualityLabel(level, idx)}
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