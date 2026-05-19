<script lang="ts">
  import { onMount, tick } from 'svelte';
  import ChatComposer from './chat-composer.svelte';
  import EmotePicker from './emote-picker.svelte';
  import { getChatEmotes, type EmoteItem } from '$lib/api-client';
  import { parseChatEvent, emoteUrl, formatUnreadMessage } from './chat-utils.svelte';
  import type { ChatMessage } from './chat-utils.svelte';

  // Constants
  const AUTO_SCROLL_THRESHOLD_PX = 32;
  const UNREAD_COUNT_ZERO = 0;
  const EMPTY_STRING = '';

  interface Props {
    channelLogin: string;
    chatAvailable: boolean;
    availableEmotes?: EmoteItem[];
    onStatusChange: (status: { available: boolean; connected: boolean; message: string }) => void;
  }

  const { channelLogin, chatAvailable, availableEmotes: initialEmotes = [], onStatusChange }: Props = $props();

  // State
  let localEmotes = $state<EmoteItem[]>([]);
  let emotesLoaded = $state(false);
  let chatEvents = $state<EventSource | null>(null);
  let chatConnected = $state(false);
  let chatStatus = $state('Checking Twitch chat...');
  let chatMessages = $state<ChatMessage[]>([]);
  let unreadChatCount = $state(UNREAD_COUNT_ZERO);
  let chatSending = $state(false);
  // eslint-disable-next-line init-declarations -- Svelte bind:this requires let
  // eslint-disable-next-line prefer-const -- Svelte bind:this mutates the variable
  let chatMessagesEl = $state<HTMLDivElement | null>(null);
  // eslint-disable-next-line init-declarations -- Svelte bind:this requires let
  // eslint-disable-next-line prefer-const -- Svelte bind:this mutates the variable
  let composerRef = $state<{ insertEmote?: (code: string) => void } | null>(null);

  // Initialize emotes
  $effect(() => {
    if (initialEmotes.length > UNREAD_COUNT_ZERO && !emotesLoaded) {
      localEmotes = initialEmotes;
      emotesLoaded = true;
    }
  });

  $effect(() => {
    onStatusChange({ available: chatAvailable, connected: chatConnected, message: chatStatus });
  });

  // Scroll helpers
  const isNearBottom = (): boolean => {
    if (!chatMessagesEl) {
      return true;
    }
    const distance =
      chatMessagesEl.scrollHeight -
      chatMessagesEl.clientHeight -
      chatMessagesEl.scrollTop;
    return distance <= AUTO_SCROLL_THRESHOLD_PX;
  };

  // Chat operations
  const appendMessage = (message: ChatMessage): Promise<void> => {
    const UNREAD_INCREMENT = 1;
    const shouldStickToBottom = isNearBottom();
    chatMessages = [...chatMessages, message];

    return tick().then(() => {
      if (shouldStickToBottom && chatMessagesEl) {
        chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
        unreadChatCount = UNREAD_COUNT_ZERO;
      } else {
        unreadChatCount += UNREAD_INCREMENT;
      }
    });
  };

  const loadEmotes = (): Promise<void> => {
    if (emotesLoaded || !channelLogin) {
      return Promise.resolve();
    }

    return getChatEmotes(channelLogin).then((emotes: EmoteItem[]) => {
      localEmotes = emotes;
      emotesLoaded = true;
    });
  };

  const sendMessage = (text: string): Promise<void> => {
    const trimmed = text.trim();
    if (trimmed.length === UNREAD_COUNT_ZERO) {
      return Promise.resolve();
    }

    chatSending = true;

    return fetch('/api/chat/send', {
      body: JSON.stringify({ channel_login: channelLogin, message: trimmed }),
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    })
      .then((response: Response) => {
        if (!response.ok) {
          return response.text().then((error: string) => {
            throw new Error(error || 'Failed to send message');
          });
        }
        chatStatus = `Connected to #${channelLogin}`;
      })
      .catch((error: unknown) => {
        chatStatus = error instanceof Error ? error.message : 'Failed to send message';
        throw error;
      })
      .finally(() => {
        chatSending = false;
      });
  };

  const subscribeChat = (): Promise<void> => {
    return fetch('/api/chat/subscribe', {
      body: JSON.stringify({ channel_login: channelLogin }),
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    }).then((response: Response) => {
      if (!response.ok) {
        return response.text().then((error: string) => {
          throw new Error(error || 'Failed to subscribe to chat');
        });
      }
      chatStatus = `Connected to #${channelLogin}`;
    });
  };

  const cleanupChat = (): Promise<void> => {
    if (chatEvents) {
      chatEvents.close();
      chatEvents = null;
    }

    if (!channelLogin) {
      return Promise.resolve();
    }
    return fetch(`/api/chat/subscribe/${encodeURIComponent(channelLogin)}`, {
      body: JSON.stringify({}),
      credentials: 'same-origin',
      keepalive: true,
      method: 'DELETE',
    }).then(() => {
      // Cleanup complete
    });
  };

  // Event handlers
  const handleChatEvent = (message: ChatMessage): void => {
    appendMessage(message).catch(() => {
      // Ignore append errors
    });
  };

  const openChatEvents = (): void => {
    if (chatEvents) {
      chatEvents.close();
    }

    chatEvents = new EventSource(`/api/chat/events/${encodeURIComponent(channelLogin)}`, {
      withCredentials: true,
    });

    chatEvents.addEventListener('open', () => {
      chatConnected = true;
      chatStatus = `Connected to #${channelLogin}`;
    });

    chatEvents.addEventListener('error', () => {
      chatConnected = false;
      chatStatus = 'Chat reconnecting...';
    });

    chatEvents.addEventListener('chat', (event: Event) => {
      const messageEvent = event as MessageEvent<string>;
      const message = parseChatEvent(messageEvent.data);
      if (message) {
        handleChatEvent(message);
      }
    });
  };

  const setupChat = (): Promise<void> => {
    chatStatus = 'Connecting to chat...';
    chatConnected = false;

    return subscribeChat()
      .then(() => {
        openChatEvents();
        void loadEmotes().catch(() => {
          // Ignore emote loading errors
        });
      })
      .catch(() => {
        chatStatus = 'Chat unavailable';
      });
  };

  const handleScroll = (): void => {
    if (isNearBottom()) {
      unreadChatCount = UNREAD_COUNT_ZERO;
    }
  };

  const jumpToLatest = (): void => {
    if (chatMessagesEl) {
      chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
      unreadChatCount = UNREAD_COUNT_ZERO;
    }
  };

  const onComposerSelect = (code: string): void => {
    composerRef?.insertEmote?.(code);
  };

  // Mount
  onMount(() => {
    if (chatAvailable) {
      Promise.resolve().then(() => {
        void setupChat().catch(() => {
          // Error handled in setupChat
        });
      });
    }
    return (): void => {
      void cleanupChat().catch(() => {
        // Ignore cleanup errors
      });
    };
  });
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
      {#if chatMessages.length === UNREAD_COUNT_ZERO}
        <p class="chat-empty">Waiting for messages...</p>
      {/if}
      {#each chatMessages as message (message.id)}
        <div class="chat-message" class:notice={message.kind === 'notice'}>
          <span class="sender" style:color={message.sender_color || undefined}>{message.sender_display_name}</span>
          <span class="content">
            {#if message.parts.length > UNREAD_COUNT_ZERO}
              {#each message.parts as part, index (`${message.id}-${index}`)}
                {#if part.kind === 'emote'}
                  <img
                    class="emote"
                    src={part.image_url || emoteUrl(part.id || EMPTY_STRING)}
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

    {#if unreadChatCount > UNREAD_COUNT_ZERO}
      <button type="button" class="unread-pill" onclick={jumpToLatest}>
        {formatUnreadMessage(unreadChatCount)}
      </button>
    {/if}

    <div class="chat-form">
      <EmotePicker
        availableEmotes={localEmotes}
        onSelect={onComposerSelect}
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
