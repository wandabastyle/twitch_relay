<script lang="ts">
  import { onMount, tick } from 'svelte';

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
  const RELAY_PARAM = '1';

  interface Props {
    ticket: string;
  }

  const { ticket }: Props = $props();

  let channelLogin = $state('');
  let appVersion = $state('');
  let manifestUrl = $state('');
  let watchLoading = $state(true);
  let watchError = $state<string>();
  let playbackError = $state<string>();
  let chatAvailable = $state(false);
  let availableEmotes = $state<EmoteItem[]>([]);
  let twitchStatusChecked = $state(false);

  const connectTwitchUrl = getTwitchConnectUrl();

  const MIN_MESSAGE_LENGTH = 0;

  const readMessage = (err: unknown, fallback: string): string => {
    if (err instanceof Error && err.message.trim().length > MIN_MESSAGE_LENGTH) {
      return err.message;
    }
    return fallback;
  };

  const isRelayForced = (): boolean =>
    typeof globalThis !== 'undefined' && new URLSearchParams(globalThis.location.search).get('relay') === RELAY_PARAM;

  const applySession = (session: { channel: string; app_version: string; manifest_url: string }): void => {
    channelLogin = session.channel;
    appVersion = session.app_version;
    manifestUrl = session.manifest_url;
  };

  const resetWatchState = (): void => {
    watchError = undefined;
    watchLoading = true;
    playbackError = undefined;
  };

  const loadEmotes = async (): Promise<void> => {
    if (!channelLogin) {
      return;
    }
    availableEmotes = await getChatEmotes(channelLogin);
  };

  const setupChat = async (): Promise<void> => {
    chatAvailable = false;
    twitchStatusChecked = false;

    try {
      const twitchStatus = await getTwitchStatus();
      twitchStatusChecked = true;
      if (!twitchStatus.connected) {
        chatAvailable = false;
        return;
      }

      chatAvailable = true;
      await loadEmotes();
    } catch {
      chatAvailable = false;
      twitchStatusChecked = true;
    }
  };

  const initializeWatchPage = async (): Promise<void> => {
    resetWatchState();

    const forceRelay = isRelayForced();
    try {
      const session = await getWatchSession(ticket, forceRelay);
      applySession(session);

      watchLoading = false;
      await tick();

      // Setup chat
      await setupChat();
    } catch (error) {
      watchError = readMessage(error, ERROR_SESSION_FAILED);
      watchLoading = false;
    }
  };

  const handleChatStatusChange = (status: { available: boolean; connected: boolean; message: string }): void => {
    chatAvailable = status.available;
  };

  onMount(() => {
    if (!ticket) {
      watchError = ERROR_MISSING_TICKET;
      watchLoading = false;
      return;
    }

    initializeWatchPage();
  });
</script>

<section class="watch-page">
  <header class="watch-page-header">
    <div class="watch-page-meta">
      <strong>{channelLogin || 'stream'}</strong>
      <span>via Twitch Relay{appVersion ? ` · v${appVersion}` : ''}</span>
    </div>
    <div class="watch-page-actions">
      <button type="button" class="ui-nav-chip" onclick={() => navigate('/twitch')}>Back to channels</button>
      {#if twitchStatusChecked && !chatAvailable}
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
