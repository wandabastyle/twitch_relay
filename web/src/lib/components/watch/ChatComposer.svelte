<script lang="ts">
  import { tick } from 'svelte';

  interface EmoteItem {
    id: string;
    code: string;
    image_url: string;
  }

  interface ActiveEmoteQuery {
    query: string;
    start: number;
    end: number;
  }

  interface Props {
    availableEmotes: EmoteItem[];
    disabled?: boolean;
    onSubmit: (text: string) => void;
  }

  let { availableEmotes, disabled = false, onSubmit }: Props = $props();

  let composerEl = $state<HTMLDivElement | null>(null);
  let text = $state('');
  let suggestionsOpen = $state(false);
  let suggestionIndex = $state(0);
  let suggestionItems = $state<EmoteItem[]>([]);

  function handleInput(): void {
    normalizeInput();
    refreshSuggestions();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && !event.shiftKey) {
      if (suggestionsOpen && suggestionItems.length > 0) {
        event.preventDefault();
        const selected = suggestionItems[suggestionIndex];
        const range = findActiveEmoteQuery();
        if (selected && range) {
          insertEmote(selected.code, range);
        }
        closeSuggestions();
      } else {
        event.preventDefault();
        submit();
      }
      return;
    }

    if (!suggestionsOpen || suggestionItems.length === 0) {
      if (event.key === 'Escape') {
        event.preventDefault();
      }
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      suggestionIndex = (suggestionIndex + 1) % suggestionItems.length;
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      suggestionIndex =
        (suggestionIndex - 1 + suggestionItems.length) % suggestionItems.length;
      return;
    }

    if (event.key === 'Tab' || event.key === 'Enter') {
      event.preventDefault();
      const selected = suggestionItems[suggestionIndex];
      const range = findActiveEmoteQuery();
      if (selected && range) {
        insertEmote(selected.code, range);
      }
      closeSuggestions();
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      closeSuggestions();
    }
  }

  function handlePaste(event: ClipboardEvent): void {
    event.preventDefault();
    const pasted = event.clipboardData?.getData('text/plain') || '';
    const cleaned = pasted.replace(/[\r\n]+/g, ' ');
    if (!cleaned) return;

    const selection = window.getSelection();
    if (!selection || !composerEl) {
      text = (text + cleaned).slice(0, 500);
    } else {
      // Insert at cursor position
      const before = text.slice(0, selection.anchorOffset);
      const after = text.slice(selection.focusOffset);
      text = (before + cleaned + after).slice(0, 500);
    }

    tick().then(() => {
      refreshSuggestions();
    });
  }

  function normalizeInput(): void {
    if (!composerEl) return;
    // Remove line breaks and limit length
    text = text.replace(/[\r\n]+/g, ' ').slice(0, 500);
  }

  function refreshSuggestions(): void {
    const active = findActiveEmoteQuery();
    if (!active) {
      closeSuggestions();
      return;
    }

    const query = active.query.toLowerCase();
    const ranked = availableEmotes
      .map((item) => ({ item, score: scoreEmote(item.code, query) }))
      .filter((entry) => entry.score < 99)
      .sort((a, b) => {
        if (a.score !== b.score) return a.score - b.score;
        return a.item.code.toLowerCase().localeCompare(b.item.code.toLowerCase());
      })
      .slice(0, 10)
      .map((entry) => entry.item);

    if (ranked.length === 0) {
      closeSuggestions();
      return;
    }

    suggestionsOpen = true;
    suggestionItems = ranked;
    suggestionIndex = Math.min(suggestionIndex, ranked.length - 1);
  }

  function findActiveEmoteQuery(): ActiveEmoteQuery | null {
    const match = text.match(/(^|\s):([A-Za-z0-9_]{2,})$/);
    if (!match) return null;
    const query = match[2];
    return {
      query,
      start: text.length - query.length - 1,
      end: text.length,
    };
  }

  function scoreEmote(code: string, query: string): number {
    const loweredCode = code.toLowerCase();
    const loweredQuery = query.toLowerCase();
    if (loweredCode === loweredQuery) return 0;
    if (loweredCode.startsWith(loweredQuery)) return 1;
    if (loweredCode.includes(loweredQuery)) return 2;
    return 99;
  }

  function insertEmote(code: string, range: ActiveEmoteQuery | null): void {
    const safeCode = code.trim();
    if (!safeCode) return;

    if (range) {
      const before = text.slice(0, range.start);
      const after = text.slice(range.end);
      text = `${before}${safeCode} ${after}`;
    } else {
      text = `${text}${safeCode} `;
    }

    tick().then(() => {
      placeCaretAtEnd();
    });
  }

  function placeCaretAtEnd(): void {
    if (!composerEl) return;
    composerEl.focus();

    const range = document.createRange();
    range.selectNodeContents(composerEl);
    range.collapse(false);

    const selection = window.getSelection();
    if (!selection) return;
    selection.removeAllRanges();
    selection.addRange(range);
  }

  function closeSuggestions(): void {
    suggestionsOpen = false;
    suggestionItems = [];
    suggestionIndex = 0;
  }

  function submit(): void {
    const trimmed = text.trim();
    if (!trimmed || disabled) return;
    onSubmit(trimmed);
    text = '';
    closeSuggestions();
  }

  function handleSuggestionClick(item: EmoteItem, event: MouseEvent): void {
    event.preventDefault();
    const range = findActiveEmoteQuery();
    insertEmote(item.code, range);
    closeSuggestions();
  }
</script>

<div class="composer-container">
  <div
    bind:this={composerEl}
    class="composer-input"
    class:is-disabled={disabled}
    contenteditable={!disabled}
    role="textbox"
    tabindex={disabled ? -1 : 0}
    aria-label="Send a message"
    data-placeholder="Send a message"
    aria-disabled={disabled}
    oninput={handleInput}
    onpaste={handlePaste}
    onkeydown={handleKeydown}
  >
    {text}
  </div>

  {#if suggestionsOpen}
    <div class="suggestions">
      {#each suggestionItems as item, index (item.id)}
        <button
          type="button"
          class="suggestion-item"
          class:active={index === suggestionIndex}
          onmousedown={(e) => handleSuggestionClick(item, e)}
        >
          <img src={item.image_url} alt={item.code} loading="lazy" decoding="async" />
          <span>{item.code}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .composer-container {
    position: relative;
    flex: 1 1 0%;
  }

  .composer-input {
    box-sizing: border-box;
    width: 100%;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--fg);
    border-radius: 6px;
    padding: 0.35rem 0.55rem;
    height: 2.2rem;
    min-height: 2.2rem;
    max-height: 2.2rem;
    line-height: 1.2;
    white-space: nowrap;
    overflow-x: auto;
    overflow-y: hidden;
    word-break: normal;
    outline: none;
  }

  .composer-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--ring);
  }

  .composer-input:empty::before {
    content: attr(data-placeholder);
    color: var(--muted);
    pointer-events: none;
  }

  .composer-input.is-disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .suggestions {
    position: absolute;
    left: 0;
    right: 0;
    bottom: calc(100% + 0.42rem);
    border: 1px solid var(--border);
    background: var(--bg-soft);
    border-radius: 6px;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.42);
    max-height: 180px;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
    z-index: 45;
  }

  .suggestions::-webkit-scrollbar {
    display: none;
  }

  .suggestion-item {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.35rem 0.5rem;
    cursor: pointer;
    color: var(--fg);
    font-size: 0.86rem;
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
  }

  .suggestion-item:hover,
  .suggestion-item.active {
    background: var(--surface);
  }

  .suggestion-item img {
    height: 22px;
    width: auto;
  }
</style>
