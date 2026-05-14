<script lang="ts">
  import { onMount } from 'svelte';
  import { tick } from 'svelte';
  import ChatComposer from './ChatComposer.svelte';
  import EmotePicker from './EmotePicker.svelte';
  import { getChatEmotes, type EmoteItem } from '$lib/api-client';

  interface ChatPart {
    kind: 'text' | 'emote';
    text?: string;
    id?: string;
    code?: string;
    image_url?: string;
  }

  interface ChatMessage {
    id: string;
    kind: 'message' | 'notice';
    sender_display_name: string;
    sender_color: string | null;
    text: string;
    parts: ChatPart[];
  }

  interface Props {
    channelLogin: string;
    chatAvailable: boolean;
    availableEmotes?: EmoteItem[];
    onStatusChange: (status: { available: boolean; connected: boolean; message: string }) => void;
  }

  let { channelLogin, chatAvailable, availableEmotes: initialEmotes = [], onStatusChange }: Props = $props();

  // Local state for emotes - don't mutate props
  let localEmotes = $state<EmoteItem[]>(initialEmotes);
  let emotesLoaded = $state(false);

  // Update when initialEmotes changes (prop updates from parent)
  $effect(() => {
    if (initialEmotes.length > 0) {
      localEmotes = initialEmotes;
      emotesLoaded = true;
    }
  });

  let chatEvents = $state<EventSource | null>(null);
  let chatConnected = $state(false);
  let chatStatus = $state('Checking Twitch chat...');
  let chatMessages = $state<ChatMessage[]>([]);
  let unreadChatCount = $state(0);
  let chatSending = $state(false);
  let chatMessagesEl = $state<HTMLDivElement | null>(null);

  // Reference to ChatComposer component for calling insertEmote
  let composerRef = $state<ReturnType<typeof ChatComposer> | null>(null);

  const AUTO_SCROLL_THRESHOLD_PX = 32;

  $effect(() => {
    onStatusChange({ available: chatAvailable, connected: chatConnected, message: chatStatus });
  });

  onMount(() => {
    if (chatAvailable) {
      void setupChat();
    }
    return () => void cleanupChat();
  });

  async function setupChat(): Promise<void> {
    chatStatus = 'Connecting to chat...';
    chatConnected = false;

    try {
      await subscribeChat();
      openChatEvents();
      void loadEmotes();
    } catch (error) {
      chatStatus = 'Chat unavailable';
      console.error('Chat setup error:', error);
    }
  }

  async function cleanupChat(): Promise<void> {
    if (chatEvents) {
      chatEvents.close();
      chatEvents = null;
    }

    if (!channelLogin) return;
    try {
      await fetch(`/api/chat/subscribe/${encodeURIComponent(channelLogin)}`, {
        method: 'DELETE',
        credentials: 'same-origin',
        keepalive: true,
      });
    } catch {
      // no-op
    }
  }

  async function subscribeChat(): Promise<void> {
    const response = await fetch('/api/chat/subscribe', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      credentials: 'same-origin',
      body: JSON.stringify({ channel_login: channelLogin }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || 'Failed to subscribe to chat');
    }

    chatStatus = `Connected to #${channelLogin}`;
  }

  function openChatEvents(): void {
    if (chatEvents) {
      chatEvents.close();
    }

    chatEvents = new EventSource(`/api/chat/events/${encodeURIComponent(channelLogin)}`, {
      withCredentials: true,
    });

    chatEvents.onopen = () => {
      chatConnected = true;
      chatStatus = `Connected to #${channelLogin}`;
    };

    chatEvents.onerror = () => {
      chatConnected = false;
      chatStatus = 'Chat reconnecting...';
    };

    chatEvents.addEventListener('chat', (event) => {
      const message = parseChatEvent((event as MessageEvent<string>).data);
      if (!message) return;
      void appendMessage(message);
    });
  }

  async function appendMessage(message: ChatMessage): Promise<void> {
    const shouldStickToBottom = isNearBottom();
    chatMessages = [...chatMessages, message];
    await tick();

    if (shouldStickToBottom && chatMessagesEl) {
      chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
      unreadChatCount = 0;
    } else {
      unreadChatCount += 1;
    }
  }

  async function loadEmotes(): Promise<void> {
    if (emotesLoaded || !channelLogin) return;

    const emotes = await getChatEmotes(channelLogin);
    localEmotes = emotes;
    emotesLoaded = true;
  }

  async function sendMessage(text: string): Promise<void> {
    const trimmed = text.trim();
    if (!trimmed) return;

    chatSending = true;
    try {
      const response = await fetch('/api/chat/send', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        credentials: 'same-origin',
        body: JSON.stringify({
          channel_login: channelLogin,
          message: trimmed,
        }),
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || 'Failed to send message');
      }

      chatStatus = `Connected to #${channelLogin}`;
    } catch (error) {
      chatStatus = error instanceof Error ? error.message : 'Failed to send message';
    } finally {
      chatSending = false;
    }
  }

  function handleScroll(): void {
    if (isNearBottom()) {
      unreadChatCount = 0;
    }
  }

  function jumpToLatest(): void {
    if (!chatMessagesEl) return;
    chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
    unreadChatCount = 0;
  }

  function isNearBottom(): boolean {
    if (!chatMessagesEl) return true;
    const distance =
      chatMessagesEl.scrollHeight -
      chatMessagesEl.clientHeight -
      chatMessagesEl.scrollTop;
    return distance <= AUTO_SCROLL_THRESHOLD_PX;
  }

  function parseChatEvent(raw: string): ChatMessage | null {
    try {
      const payload = JSON.parse(raw) as Record<string, unknown>;
      if (!payload || (payload.kind !== 'message' && payload.kind !== 'notice')) {
        return null;
      }

      const sender_display_name =
        typeof payload.sender_display_name === 'string' &&
        payload.sender_display_name.trim().length > 0
          ? payload.sender_display_name
          : typeof payload.sender_login === 'string'
            ? payload.sender_login
            : 'system';

      const sender_color =
        payload.kind === 'message' &&
        typeof payload.sender_color === 'string' &&
        payload.sender_color.trim().length > 0
          ? payload.sender_color
          : null;

      const parts: ChatPart[] = [];
      if (Array.isArray(payload.parts)) {
        for (const part of payload.parts) {
          const p = part as Record<string, unknown>;
          if (!p || typeof p.kind !== 'string') continue;

          if (p.kind === 'text' && typeof p.text === 'string') {
            parts.push({ kind: 'text', text: p.text });
          } else if (
            p.kind === 'emote' &&
            typeof p.id === 'string' &&
            typeof p.code === 'string'
          ) {
            parts.push({
              kind: 'emote',
              id: p.id,
              code: p.code,
              image_url: typeof p.image_url === 'string' ? p.image_url : undefined,
            });
          }
        }
      }

      return {
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        kind: payload.kind,
        sender_display_name,
        sender_color,
        text: typeof payload.text === 'string' ? payload.text : '',
        parts,
      };
    } catch {
      return null;
    }
  }

  function normalizeEmoteCode(code: string): string {
    return code.trim();
  }

  function emoteUrl(emoteId: string): string {
    return `https://static-cdn.jtvnw.net/emoticons/v2/${encodeURIComponent(emoteId)}/default/dark/2.0`;
  }
</script>

<div class="chat-panel">
  <div class="chat-header">
    <strong>Chat</strong>
    <span class:status-live={chatConnected}>{chatStatus}</span>
  </div>

  {#if !chatAvailable}
    <div class="chat-offline">
      <p class="muted">Connect Twitch to read and send messages.</p>
    </div>
  {:else}
    <div class="chat-messages ui-hide-scrollbar" bind:this={chatMessagesEl} onscroll={handleScroll}>
      {#if chatMessages.length === 0}
        <p class="chat-empty">Waiting for messages...</p>
      {/if}
      {#each chatMessages as message (message.id)}
        <div class="chat-message" class:notice={message.kind === 'notice'}>
          <span class="sender" style:color={message.sender_color || undefined}>{message.sender_display_name}</span>
          <span class="content">
            {#if message.parts.length > 0}
              {#each message.parts as part, index (`${message.id}-${index}`)}
                {#if part.kind === 'emote'}
                  <img
                    class="emote"
                    src={part.image_url || emoteUrl(part.id || '')}
                    alt={part.code}
                    title={part.code}
                    loading="lazy"
                    decoding="async"
                  />
                {:else}
                  {part.text}
                {/if}
              {/each}
            {:else}
              {message.text}
            {/if}
          </span>
        </div>
      {/each}
    </div>

    {#if unreadChatCount > 0}
      <button type="button" class="unread-pill" onclick={jumpToLatest}>
        {unreadChatCount <= 1 ? '1 new message' : `${unreadChatCount > 99 ? '99+' : unreadChatCount} new messages`}
      </button>
    {/if}

    <div class="chat-form">
      <EmotePicker
        availableEmotes={localEmotes}
        onSelect={(code) => {
          composerRef?.insertEmote?.(code);
        }}
      />

      <ChatComposer
        bind:this={composerRef}
        availableEmotes={localEmotes}
        disabled={chatSending}
        onSubmit={sendMessage}
      />
    </div>
  {/if}
</div>

<style>
  .chat-panel {
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

  .chat-header {
    padding: 0.65rem 0.75rem;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.82rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.55rem;
  }

  .chat-header strong {
    color: var(--fg);
    font-size: 0.85rem;
  }

  .status-live {
    color: var(--success);
  }

  .chat-offline {
    padding: 0.85rem;
    display: grid;
    gap: 0.75rem;
  }

  .muted {
    color: var(--muted);
    margin: 0;
  }

  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 0.65rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .chat-empty {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .chat-message {
    line-height: 1.35;
    word-break: break-word;
    font-size: 0.9rem;
  }

  .chat-message .sender {
    color: var(--accent);
    font-weight: 600;
    margin-right: 0.35rem;
  }

  .chat-message.notice .sender {
    color: var(--warn);
  }

  .chat-message .content .emote {
    height: 1.6em;
    width: auto;
    vertical-align: middle;
    margin: 0 0.05em;
  }

  .unread-pill {
    position: absolute;
    left: 50%;
    bottom: 3.6rem;
    transform: translateX(-50%);
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: rgba(47, 51, 77, 0.96);
    color: var(--fg);
    border-radius: 999px;
    padding: 0.34rem 0.78rem;
    font-size: 0.95rem;
    line-height: 1;
    font-weight: 500;
    box-shadow: 0 10px 20px rgba(0, 0, 0, 0.34);
    cursor: pointer;
    z-index: 30;
    border: none;
  }

  .unread-pill:hover {
    background: rgba(59, 66, 97, 0.98);
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  }

  .chat-form {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    border-top: 1px solid var(--border);
    padding: 0.65rem;
    position: relative;
  }
</style>
