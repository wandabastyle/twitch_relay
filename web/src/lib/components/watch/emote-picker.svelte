<script lang="ts">
  import { tick } from 'svelte';

  const SEARCH_DEBOUNCE_MS = 300;

  interface EmoteItem {
    code: string;
    group_key: string;
    group_name: string;
    id: string;
    image_url: string;
  }

  interface Props {
    availableEmotes: EmoteItem[];
    children?: import('svelte').Snippet;
    onSelect: (code: string) => void;
  }

  const { availableEmotes, children, onSelect }: Props = $props();

  let pickerOpen = $state(false);
  // eslint-disable-next-line init-declarations -- Svelte bind:this requires let
  // eslint-disable-next-line prefer-const -- Svelte bind:this mutates the variable
  let searchEl = $state<HTMLInputElement>();
  let searchTerm = $state('');

  interface GroupedEmotes {
    items: EmoteItem[];
    key: string;
    title: string;
  }

  const filterEmotes = (emotes: readonly EmoteItem[], term: string): readonly EmoteItem[] =>
    term ? emotes.filter((item) => item.code.toLowerCase().includes(term)) : emotes;

  const MIN_GROUP_NAME_LENGTH = 0;

  const createGroup = (key: string, title: string): GroupedEmotes => ({ items: [], key, title });

  const buildGroups = (filtered: readonly EmoteItem[]): readonly GroupedEmotes[] => {
    const groupedMap = new Map<string, GroupedEmotes>();

    for (const item of filtered) {
      const { group_key: key = 'global', group_name: groupName } = item;
      const title = groupName.trim().length > MIN_GROUP_NAME_LENGTH ? groupName : 'Global';
      if (!groupedMap.has(key)) {
        groupedMap.set(key, createGroup(key, title));
      }
      groupedMap.get(key)?.items.push(item);
    }

    return [...groupedMap.values()];
  };

  const groupedEmotes = $derived(() => {
    const term = searchTerm.trim().toLowerCase();
    const filtered = filterEmotes(availableEmotes, term);
    return buildGroups(filtered);
  });

  const openAndFocus = (): void => {
    searchTerm = '';
    tick().then(() => searchEl?.focus());
  };

  const togglePicker = (): void => {
    pickerOpen = !pickerOpen;
    if (pickerOpen) {
      openAndFocus();
    }
  };

  const closePicker = (): void => {
    pickerOpen = false;
  };

  const handleSelect = (code: string): void => {
    onSelect(code);
    closePicker();
  };

  const handleDocumentClick = (event: MouseEvent): void => {
    const { target } = event;
    if (!target) {
      return;
    }

    const clickedInsidePopup = (target as HTMLElement).closest('.emote-popup') !== undefined;
    const clickedToggle = (target as HTMLElement).closest('.emote-toggle') !== undefined;

    if (!clickedInsidePopup && !clickedToggle) {
      closePicker();
    }
  };
</script>

<svelte:document onclick={handleDocumentClick} />

<button
  type="button"
  class="emote-toggle"
  title="Open emote picker"
  aria-label="Open emote picker"
  onclick={togglePicker}
>
  {#if children}
    {@render children()}
  {:else}
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="10"/>
      <path d="M8 14s1.5 2 4 2 4-2 4-2"/>
      <line x1="9" y1="9" x2="9.01" y2="9"/>
      <line x1="15" y1="9" x2="15.01" y2="9"/>
    </svg>
  {/if}
</button>

{#if pickerOpen}
  <div class="emote-popup">
    <input
      bind:this={searchEl}
      class="emote-search"
      type="text"
      placeholder="Search emotes"
      autocomplete="off"
      bind:value={searchTerm}
    />

    <div class="emote-groups ui-hide-scrollbar">
      {#each groupedEmotes() as group (group.key)}
        <p class="emote-group-title">{group.title}</p>
        <div class="emote-grid">
          {#each group.items as item (item.id)}
            <button
              type="button"
              class="emote-item"
              title={item.code}
              aria-label={item.code}
              onclick={() => handleSelect(item.code)}
            >
              <img src={item.image_url} alt={item.code} loading="lazy" decoding="async" />
            </button>
          {/each}
        </div>
      {/each}

      {#if groupedEmotes().length === 0}
        <div class="emote-empty">
          {#if searchTerm}
            No emotes match your search.
          {:else}
            No emotes available.
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .emote-toggle {
    box-sizing: border-box;
    width: 2.2rem;
    height: 2.2rem;
    min-width: 2.2rem;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 6px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .emote-toggle:hover {
    border-color: var(--accent);
    background: var(--surface-2);
  }

  .emote-popup {
    position: absolute;
    left: 0.65rem;
    right: 0.65rem;
    bottom: calc(100% + 0.5rem);
    background: var(--bg-soft);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    max-height: min(52vh, 420px);
    overflow: hidden;
    z-index: 40;
  }

  .emote-search {
    margin: 0.6rem;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--fg);
    border-radius: 6px;
    padding: 0.45rem 0.55rem;
    font-size: 0.9rem;
  }

  .emote-search:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--ring);
  }

  .emote-groups {
    overflow-y: auto;
    padding: 0 0.6rem 0.6rem;
  }

  .emote-group-title {
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    margin: 0.5rem 0 0.38rem;
  }

  .emote-grid {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 0.35rem;
  }

  .emote-item {
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--fg);
    min-height: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .emote-item:hover {
    background: color-mix(in srgb, var(--surface-2) 62%, transparent);
  }

  .emote-item img {
    max-height: 30px;
    max-width: 30px;
  }

  .emote-empty {
    color: var(--muted);
    font-size: 0.85rem;
    padding: 0.75rem 0.2rem;
    text-align: center;
  }
</style>
