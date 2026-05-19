<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';

  import { navigate } from '$lib/router/router.svelte';
  import {
    getChatEmotes,
    getTwitchConnectUrl,
    getTwitchStatus,
    getWatchSession,
    type EmoteItem,
  } from '$lib/api-client';
  import { Chat, VideoPlayer } from '$lib/components/watch';

  const ERROR_MISSING_TICKET = 'Missing watch ticket.';
  const ERROR_SESSION_FAILED = 'Failed to initialize watch session.';
  const ERROR_STREAM_UNAVAILABLE = 'Stream unavailable. The channel may be offline or not accessible.';
  const ERROR_CHAT_UNAVAILABLE = 'Chat unavailable';
  const STATUS_CHECK_TWITCH = 'Checking Twitch chat...';
  const STATUS_CONNECT_TWITCH = 'Connect Twitch to use chat.';
  const RELAY_PARAM = '1';

  interface Props {
    ticket: string;
  }

  const { ticket }: Props = $props();

  let channelLogin = $state('');
  let appVersion = $state('');
  let manifestUrl = $state('');
  let watchLoading = $state(true);
  let watchError = $state<string | undefined>(undefined);
  let playbackError = $state<string | undefined>(undefined);
  let attemptedRelayFallback = $state(false);

  let chatAvailable = $state(false);
  let chatConnected = $state(false);
  let chatStatus = $state(STATUS_CHECK_TWITCH);
  let availableEmotes = $state<EmoteItem[]>([]);

  const connectTwitchUrl = getTwitchConnectUrl();

  onMount(() => {
    if (!ticket) {
      watchError = ERROR_MISSING_TICKET;
      watchLoading = false;
      return;
    }

    initializeWatchPage();
  });

  const initializeWatchPage = async (): Promise<void> => {
    watchError = undefined;
    watchLoading = true;
    playbackError = undefined;

    const forceRelay =
      typeof globalThis !== 'undefined' &&
      new URLSearchParams(globalThis.location.search).get('relay') === RELAY_PARAM;
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
      watchError = readMessage(error, ERROR_SESSION_FAILED);
      watchLoading = false;
    }
  };

  const setupChat = async (): Promise<void> => {
    chatStatus = STATUS_CHECK_TWITCH;
    chatAvailable = false;

    try {
      const twitchStatus = await getTwitchStatus();
      if (!twitchStatus.connected) {
        chatAvailable = false;
        chatStatus = STATUS_CONNECT_TWITCH;
        return;
      }

      chatAvailable = true;
      loadEmotes();
    } catch (error) {
      chatAvailable = false;
      chatStatus = readMessage(error, ERROR_CHAT_UNAVAILABLE);
    }
  };

  const loadEmotes = async (): Promise<void> => {
    if (!channelLogin) {
      return;
    }
    availableEmotes = await getChatEmotes(channelLogin);
  };

  const handleFatalPlaybackError = (): void => {
    if (typeof globalThis === 'undefined') {
      playbackError = ERROR_STREAM_UNAVAILABLE;
      return;
    }

    if (!attemptedRelayFallback) {
      attemptedRelayFallback = true;
      const nextUrl = new URL(globalThis.location.href);
      nextUrl.searchParams.set('relay', RELAY_PARAM);
      globalThis.location.assign(nextUrl.toString());
      return;
    }

    playbackError = ERROR_STREAM_UNAVAILABLE;
  };

  const handleChatStatusChange = (status: { available: boolean; connected: boolean; message: string }): void => {
    chatAvailable = status.available;
    chatConnected = status.connected;
    chatStatus = status.message;
  };

  const readMessage = (err: unknown, fallback: string): string => {
    if (err instanceof Error && err.message.trim().length > 0) {
      return err.message;
    }
    return fallback;
  };

  const toObject = (value: unknown): Record<string, unknown> | null => {
    if (typeof value === 'object' && value !== null) {
      return value as Record<string, unknown>;
    }
    return null;
  };
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
        <button type="button" class="ui-nav-chip" onclick={() => globalThis.location.href = connectTwitchUrl}>Connect Twitch</button>
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
            <button type="button" class="ui-nav-chip" onclick={() => globalThis.location.href = connectTwitchUrl}>Connect Twitch</button>
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
