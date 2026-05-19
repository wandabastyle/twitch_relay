<script lang="ts">
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

  // Get plain text from composer, preserving structure
  function getComposerText(): string {
    if (!composerEl) return '';
    // Get text content (ignores images but that's OK for plain text extraction)
    return (composerEl as HTMLDivElement).innerText || '';
  }

  // Get current cursor position in text
  function getCursorPosition(): number {
    const selection = window.getSelection();
    if (!selection || !composerEl || selection.rangeCount === 0) return text.length;

    const range = selection.getRangeAt(0);
    if (!composerEl.contains(range.commonAncestorContainer)) return text.length;

    // Create a range from start of composer to cursor
    const preRange = document.createRange();
    preRange.setStart(composerEl, 0);
    preRange.setEnd(range.startContainer, range.startOffset);

    // Get length of text before cursor
    const div = document.createElement('div');
    div.appendChild(preRange.cloneContents());
    return div.innerText.length;
  }

  // Set cursor position in text
  function setCursorPosition(pos: number): void {
    if (!composerEl) return;

    const selection = window.getSelection();
    if (!selection) return;

    // Walk through nodes to find position
    let currentPos = 0;
    let targetNode: Node | null = null;
    let targetOffset = 0;

    function walkNodes(node: Node): boolean {
      if (node.nodeType === Node.TEXT_NODE) {
        const textLen = node.textContent?.length || 0;
        if (currentPos + textLen >= pos) {
          targetNode = node;
          targetOffset = pos - currentPos;
          return true;
        }
        currentPos += textLen;
      } else if (node.nodeType === Node.ELEMENT_NODE) {
        for (const child of Array.from(node.childNodes)) {
          if (walkNodes(child)) return true;
        }
      }
      return false;
    }

    walkNodes(composerEl);

    if (targetNode) {
      const range = document.createRange();
      const nodeTextLen = (targetNode as Text).textContent?.length || 0;
      range.setStart(targetNode, Math.min(targetOffset, nodeTextLen));
      range.collapse(true);

      selection.removeAllRanges();
      selection.addRange(range);
    }
  }

  function handleInput(): void {
    const newText = getComposerText();
    // Remove line breaks and limit length
    if (newText.includes('\n') || newText.includes('\r')) {
      text = newText.replace(/[\r\n]+/g, ' ').slice(0, 500);
      // Re-render with normalized text
      if (composerEl) {
        const cursorPos = getCursorPosition();
        composerEl.textContent = text;
        setCursorPosition(Math.min(cursorPos, text.length));
      }
    } else {
      text = newText.slice(0, 500);
    }
    refreshSuggestions();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      if (suggestionsOpen && suggestionItems.length > 0) {
        const selected = suggestionItems[suggestionIndex];
        const query = findActiveEmoteQuery();
        if (selected && query) {
          insertEmoteAtRange(selected.code, query);
        }
        closeSuggestions();
      } else {
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

    if (event.key === 'Tab' || (event.key === 'Enter' && event.shiftKey)) {
      event.preventDefault();
      const selected = suggestionItems[suggestionIndex];
      const query = findActiveEmoteQuery();
      if (selected && query) {
          insertEmoteAtRange(selected.code, query);
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

    const cursorPos = getCursorPosition();
    const before = text.slice(0, cursorPos);
    const after = text.slice(cursorPos);
    const newText = (before + cleaned + after).slice(0, 500);

    text = newText;
    if (composerEl) {
      composerEl.textContent = newText;
      setCursorPosition(cursorPos + cleaned.length);
    }

    refreshSuggestions();
  }

  function refreshSuggestions(): void {
    const query = findActiveEmoteQuery();
    if (!query) {
      closeSuggestions();
      return;
    }

    const search = query.query.toLowerCase();
    const ranked = availableEmotes
      .map((item) => ({ item, score: scoreEmote(item.code, search) }))
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
    const cursorPos = getCursorPosition();
    const beforeCursor = text.slice(0, cursorPos);
    const match = beforeCursor.match(/(^|\s):([A-Za-z0-9_]{2,})$/);
    if (!match) return null;
    const query = match[2];
    return {
      query,
      start: beforeCursor.length - query.length - 1,
      end: beforeCursor.length,
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

  function insertEmoteAtRange(code: string, range: ActiveEmoteQuery): void {
    const safeCode = code.trim();
    if (!safeCode) return;

    const before = text.slice(0, range.start);
    const after = text.slice(range.end);
    const newText = `${before}${safeCode} ${after}`;

    // Calculate new cursor position
    const newCursorPos = range.start + safeCode.length + 1;

    text = newText;
    if (composerEl) {
      composerEl.textContent = newText;
      setCursorPosition(Math.min(newCursorPos, newText.length));
    }

    closeSuggestions();
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
    if (composerEl) {
      composerEl.textContent = '';
    }
    closeSuggestions();
  }

  function handleSuggestionClick(item: EmoteItem, event: MouseEvent): void {
    event.preventDefault();
    const query = findActiveEmoteQuery();
    if (query) {
      insertEmoteAtRange(item.code, query);
    }
    closeSuggestions();
  }

  // Public method to insert an emote by code from outside (e.g., emote picker)
  export function insertEmote(code: string): void {
    if (!composerEl || disabled) return;

    const cursorPos = getCursorPosition();
    const before = text.slice(0, cursorPos);
    const after = text.slice(cursorPos);

    // Add space before emote if not at start and previous char isn't space
    const prefix = before.length > 0 && !before.endsWith(' ') ? ' ' : '';
    // Add space after emote
    const suffix = ' ';

    const safeCode = code.trim();
    if (!safeCode) return;

    const newText = `${before}${prefix}${safeCode}${suffix}${after}`.slice(0, 500);
    const newCursorPos = before.length + prefix.length + safeCode.length + suffix.length;

    text = newText;
    composerEl.textContent = newText;
    setCursorPosition(Math.min(newCursorPos, newText.length));
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
  ></div>

  {#if suggestionsOpen}
    <div class="suggestions ui-hide-scrollbar">
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
    z-index: 45;
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
