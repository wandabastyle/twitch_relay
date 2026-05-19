<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { navigate } from '$lib/router/router.svelte';
  import { getTwitchConnectUrl, getTwitchStatus, getWatchSession, getChatEmotes } from '$lib/api-client';
  import { VideoPlayer, Chat } from '$lib/components/watch';
  import type { EmoteItem } from '$lib/api-client';

  interface Props {
    ticket: string;
  }

  let { ticket }: Props = $props();

  let channelLogin = $state('');
  let appVersion = $state('');
  let manifestUrl = $state('');
  let watchLoading = $state(true);
  let watchError = $state<string | null>(null);
  let playbackError = $state<string | null>(null);
  let attemptedRelayFallback = $state(false);

  let chatAvailable = $state(false);
  let chatConnected = $state(false);
  let chatStatus = $state('Checking Twitch chat...');
  let availableEmotes = $state<EmoteItem[]>([]);

  const connectTwitchUrl = getTwitchConnectUrl();

  onMount(() => {
    if (!ticket) {
      watchError = 'Missing watch ticket.';
      watchLoading = false;
      return;
    }

    void initializeWatchPage();
  });

  async function initializeWatchPage(): Promise<void> {
    watchError = null;
    watchLoading = true;
    playbackError = null;

    const forceRelay =
      typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('relay') === '1';
    attemptedRelayFallback = forceRelay;

    try {
      const session = await getWatchSession(ticket, forceRelay);
      channelLogin = session.channel;
      appVersion = session.app_version;
      manifestUrl = session.manifest_url;
      attemptedRelayFallback = session.relay;

      watchLoading = false;
      await tick();

      // Setup chat
      await setupChat();
    } catch (error) {
      watchError = readMessage(error, 'Failed to initialize watch session.');
      watchLoading = false;
    }
  }

  async function setupChat(): Promise<void> {
    chatStatus = 'Checking Twitch chat...';
    chatAvailable = false;

    try {
      const twitchStatus = await getTwitchStatus();
      if (!twitchStatus.connected) {
        chatAvailable = false;
        chatStatus = 'Connect Twitch to use chat.';
        return;
      }

      chatAvailable = true;
      void loadEmotes();
    } catch (error) {
      chatAvailable = false;
      chatStatus = readMessage(error, 'Chat unavailable');
    }
  }

  async function loadEmotes(): Promise<void> {
    if (!channelLogin) return;
    availableEmotes = await getChatEmotes(channelLogin);
  }

  function handleFatalPlaybackError(): void {
    if (typeof window === 'undefined') {
      playbackError = 'Stream unavailable. The channel may be offline or not accessible.';
      return;
    }

    if (!attemptedRelayFallback) {
      attemptedRelayFallback = true;
      const nextUrl = new URL(window.location.href);
      nextUrl.searchParams.set('relay', '1');
      window.location.assign(nextUrl.toString());
      return;
    }

    playbackError = 'Stream unavailable. The channel may be offline or not accessible.';
  }

  function handleChatStatusChange(status: { available: boolean; connected: boolean; message: string }): void {
    chatAvailable = status.available;
    chatConnected = status.connected;
    chatStatus = status.message;
  }

  function readMessage(error: unknown, fallback: string): string {
    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }
    return fallback;
  }

  function toObject(value: unknown): Record<string, unknown> | null {
    return typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : null;
  }
</script>

<section class="watch-page">
  <header class="watch-page-header">
    <div class="watch-page-meta">
      <strong>{channelLogin || 'stream'}</strong>
      <span>via Twitch Relay{appVersion ? ` · v${appVersion}` : ''}</span>
    </div>
    <div class="watch-page-actions">
      <button type="button" class="ui-nav-chip" onclick={() => navigate('/twitch')}>Back to channels</button>
      {#if !chatAvailable}
        <button type="button" class="ui-nav-chip" onclick={() => window.location.href = connectTwitchUrl}>Connect Twitch</button>
      {/if}
    </div>
  </header>

  {#if watchLoading}
    <div class="watch-loading-state">
      <p class="ui-muted">Loading watch session...</p>
    </div>
  {:else if watchError}
    <div class="watch-loading-state">
      <p class="ui-error">{watchError}</p>
    </div>
  {:else}
    <div class="watch-layout">
      <section class="watch-player-panel">
        <VideoPlayer
          {manifestUrl}
          onError={(msg) => playbackError = msg}
        />

        {#if playbackError}
          <p class="ui-error">{playbackError}</p>
        {/if}
      </section>

      <aside class="watch-chat-panel">
        {#if !chatAvailable}
          <div class="chat-offline">
            <p class="ui-muted">Connect Twitch to read and send messages.</p>
            <button type="button" class="ui-nav-chip" onclick={() => window.location.href = connectTwitchUrl}>Connect Twitch</button>
          </div>
        {:else}
          <Chat
            {channelLogin}
            {chatAvailable}
            {availableEmotes}
            onStatusChange={handleChatStatusChange}
          />
        {/if}
      </aside>
    </div>
  {/if}
</section>

<style>
  .watch-page {
    min-height: 100dvh;
    height: 100dvh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .watch-page-header {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .watch-page-meta {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    min-width: 0;
  }

  .watch-page-meta strong {
    font-size: 1rem;
    font-weight: 700;
    text-transform: lowercase;
    color: var(--fg);
  }

  .watch-page-meta span {
    font-size: 0.82rem;
    color: var(--muted);
  }

  .watch-page-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }

  .watch-loading-state {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 1.2rem;
  }

  .watch-layout {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) clamp(280px, 19vw, 380px);
    align-items: stretch;
    justify-content: stretch;
    gap: clamp(8px, 1.2vw, 16px);
    padding: clamp(8px, 1.2vw, 16px);
  }

  .watch-player-panel {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .watch-chat-panel {
    width: 100%;
    min-width: 0;
    min-height: 0;
    height: 100%;
    border: 1px solid var(--border);
    background: var(--bg-soft);
    display: flex;
    flex-direction: column;
    position: relative;
  }

  .chat-offline {
    padding: 0.85rem;
    display: grid;
    gap: 0.75rem;
  }

  @media (max-width: 900px) {
    .watch-page {
      height: auto;
      overflow: visible;
    }

    .watch-layout {
      display: flex;
      flex-direction: column;
    }

    .watch-chat-panel {
      width: 100%;
      min-width: 0;
      min-height: 0;
      height: 38vh;
    }
  }
</style>
