<script lang="ts">
  import { onMount } from 'svelte';

  interface HlsLevel {
    bitrate: number;
    height: number;
    name?: string;
  }

  interface Props {
    manifestUrl: string;
    onError: (message: string) => void;
  }

  const { manifestUrl, onError }: Props = $props();

  let currentPlayingLevel = $state(-1);
  let hlsInstance = $state<Hls | undefined>();
  let hlsLevels = $state<HlsLevel[]>([]);
  let liveButtonIsLive = $state(true);
  let playerEl = $state<HTMLVideoElement | undefined>();
  let qualityLevel = $state(-1);
  let qualityMenuOpen = $state(false);
  let userSelectedAuto = $state(true);

  const RESUME_ENTER_LIVE_SECS = 5.5;
  const RESUME_EXIT_LIVE_SECS = 7.5;

  onMount(() => {
    setupPlayer().catch(() => {});
    return () => {
      cleanupPlayer();
    };
  });

  const setupPlayer = async (): Promise<void> => {
    if (!playerEl) {
      return;
    }

    const hlsLoaded = await ensureHlsLoaded('/hls.js');
    if (!hlsLoaded) {
      onError('Failed to load HLS player.');
      return;
    }

    if ('Hls' in globalThis && (globalThis as unknown as { Hls: typeof Hls }).Hls.isSupported()) {
      const HlsClass = (globalThis as unknown as { Hls: typeof Hls }).Hls;
      hlsInstance = new HlsClass({
        abrEwmaFastLive: 3.0,
        abrEwmaSlowLive: 9.0,
        backBufferLength: 15,
        capLevelToPlayerSize: true,
        fragLoadingMaxRetry: 5,
        fragLoadingRetryDelay: 750,
        fragLoadingTimeOut: 20_000,
        levelLoadingMaxRetry: 3,
        levelLoadingRetryDelay: 750,
        levelLoadingTimeOut: 15_000,
        liveMaxLatencyDuration: 14,
        liveSyncDuration: 6,
        lowLatencyMode: true,
        manifestLoadingMaxRetry: 3,
        manifestLoadingRetryDelay: 750,
        manifestLoadingTimeOut: 15_000,
        maxBufferLength: 20,
        maxLiveSyncPlaybackRate: 1.1,
        maxMaxBufferLength: 45,
        startLevel: -1,
        startPosition: -6,
      });

      hlsInstance.currentLevel = -1;
      qualityLevel = -1;
      currentPlayingLevel = -1;
      userSelectedAuto = true;

      hlsInstance.on(HlsClass.Events.MANIFEST_PARSED, (_event: string, data: unknown) => {
        const parsed = toObject(data);
        let levels: HlsLevel[] = [];
        if (Array.isArray(parsed?.levels)) {
          levels = parsed.levels.filter((item): item is HlsLevel => {
        const obj = toObject(item);
            return obj !== undefined &&
              typeof obj.height === 'number' &&
              typeof obj.bitrate === 'number';
          });
        }
        hlsLevels = levels;
      });

      hlsInstance.on(HlsClass.Events.LEVEL_SWITCHED, (_event: string, data: unknown) => {
        const parsed = toObject(data);
        let level = -1;
        if (typeof parsed?.level === 'number') {
          level = parsed.level;
        }
        currentPlayingLevel = level;
        if (userSelectedAuto) {
          qualityLevel = -1;
        }
      });

      hlsInstance.on(HlsClass.Events.ERROR, (_event: string, data: unknown) => {
        const parsed = toObject(data);
        if (parsed?.fatal === true) {
          onError('Stream unavailable. The channel may be offline or not accessible.');
        }
      });

      hlsInstance.loadSource(manifestUrl);
      hlsInstance.attachMedia(playerEl);
    } else if (playerEl.canPlayType('application/vnd.apple.mpegurl')) {
      playerEl.src = manifestUrl;
    } else {
      onError('Your browser does not support HLS playback.');
      return;
    }

    playerEl.addEventListener('timeupdate', updateGoLiveState);
    playerEl.addEventListener('loadedmetadata', updateGoLiveState);
    playerEl.addEventListener('durationchange', updateGoLiveState);
    playerEl.addEventListener('error', () => {
      onError('Stream unavailable. The channel may be offline or not accessible.');
    });
  }

  const cleanupPlayer = (): void => {
    if (playerEl) {
      playerEl.removeEventListener('timeupdate', updateGoLiveState);
      playerEl.removeEventListener('loadedmetadata', updateGoLiveState);
      playerEl.removeEventListener('durationchange', updateGoLiveState);
    }

    if (hlsInstance) {
      hlsInstance.destroy();
      hlsInstance = undefined;
    }
  }

  const updateGoLiveState = (): void => {
    if (!playerEl || playerEl.seekable.length <= 0) {
      liveButtonIsLive = true;
      return;
    }

    const end = playerEl.seekable.end(playerEl.seekable.length - 1);
    const lag = Math.max(0, end - playerEl.currentTime);

    if (liveButtonIsLive) {
      if (lag > RESUME_EXIT_LIVE_SECS) {
        liveButtonIsLive = false;
      }
    } else if (lag < RESUME_ENTER_LIVE_SECS) {
      liveButtonIsLive = true;
    }
  }

  const goLive = (): void => {
    if (!playerEl || liveButtonIsLive) {
      return;
    }

    if (hlsInstance && Number.isFinite(hlsInstance.liveSyncPosition)) {
      playerEl.currentTime = hlsInstance.liveSyncPosition as number;
    } else if (playerEl.seekable.length > 0) {
      playerEl.currentTime = playerEl.seekable.end(playerEl.seekable.length - 1);
    }
    updateGoLiveState();
  }

  const qualityLabel = (level: HlsLevel, idx: number): string => {
    // Use name from manifest if available (for Source), otherwise use height
    if (level.name === 'Source' || idx === 0 && hlsLevels.length > 1 && level.height >= 1080) {
      let bitrate = '';
      if (level.bitrate > 0) {
        bitrate = ` (${(level.bitrate / 1_000_000).toFixed(1)} Mbps)`;
      }
      return `Source${bitrate}`;
    }
    let bitrate = '';
    if (level.bitrate > 0) {
      bitrate = ` (${(level.bitrate / 1_000_000).toFixed(1)} Mbps)`;
    }
    return `${level.height}p${bitrate}`;
  }

  const selectedQualityLabel = (): string => {
    if (qualityLevel === -1) {
      if (currentPlayingLevel >= 0 && hlsLevels[currentPlayingLevel]) {
        return `Auto (${hlsLevels[currentPlayingLevel].height}p)`;
      }
      return 'Auto';
    }

    const level = hlsLevels[qualityLevel];
    if (level) {
      const isSource = level.name === 'Source' || qualityLevel === 0 && hlsLevels.length > 1 && level.height >= 1080;
      if (isSource) {
        return 'Source';
      }
      return `${level.height}p`;
    }

    return 'Manual';
  }

  const setQuality = (level: number): void => {
    if (!hlsInstance) {
      return;
    }
    hlsInstance.currentLevel = level;
    qualityLevel = level;
    userSelectedAuto = level === -1;
  }

  const selectQuality = (level: number): void => {
    setQuality(level);
    qualityMenuOpen = false;
  }

  const toggleQualityMenu = (): void => {
    qualityMenuOpen = !qualityMenuOpen;
  }

  const ensureHlsLoaded = async (path: string): Promise<boolean> => {
    if (typeof globalThis === 'undefined') {
      return false;
    }
    if ('Hls' in globalThis) {
      return true;
    }

    const existing = document.querySelector<HTMLScriptElement>(`script[src="${path}"]`);
    if (existing) {
      await waitForHls();
      return 'Hls' in globalThis;
    }

    const script = document.createElement('script');
    script.src = path;
    script.async = true;

    const loaded = await new Promise<boolean>((resolve) => {
      script.addEventListener('load', () => resolve(true));
      script.addEventListener('error', () => resolve(false));
      document.head.append(script);
    });

    if (!loaded) {
      return false;
    }
    await waitForHls();
    return 'Hls' in globalThis;
  }

  const waitForHls = async (): Promise<void> => {
    let attempts = 0;
    while (typeof globalThis !== 'undefined' && !('Hls' in globalThis) && attempts < 50) {
      await new Promise((resolve) => setTimeout(resolve, 100));
      attempts += 1;
    }
  }

  const toObject = (value: unknown): Record<string, unknown> | undefined => {
    if (typeof value === 'object' && value !== undefined) {
      return value as Record<string, unknown>;
    }
    return undefined;
  }

  const handleDocumentClick = (event: MouseEvent): void => {
    const target = event.target as HTMLElement;
    const qualityMenu = document.querySelector('.watch-overlay-quality-menu');
    const qualityBtn = document.querySelector('.watch-overlay-btn.quality-btn');

    if (qualityMenuOpen && qualityMenu && qualityBtn) {
      if (!qualityMenu.contains(target) && !qualityBtn.contains(target)) {
        qualityMenuOpen = false;
      }
    }
  }
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
          class:active={qualityLevel === -1}
          onclick={() => selectQuality(-1)}
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
