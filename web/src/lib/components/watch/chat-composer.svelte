<script lang="ts">
  // Constants
  const MAX_TEXT_LENGTH = 500;
  const MAX_SUGGESTIONS = 10;
  const MIN_EMOTE_QUERY_LENGTH = 2;
  const SCORE_EXACT_MATCH = 0;
  const SCORE_STARTS_WITH = 1;
  const SCORE_INCLUDES = 2;
  const SCORE_NO_MATCH = 99;
  const SUGGESTION_OFFSET_REM = 0.42;
  const SHADOW_BLUR_PX = 3;
  const SHADOW_SPREAD_PX = 20;
  const SUGGESTION_IMG_HEIGHT_PX = 22;
  const MAX_SUGGESTION_HEIGHT_PX = 180;
  const TAB_INDEX_DISABLED = -1;
  const TAB_INDEX_ENABLED = 0;
  const CURSOR_POS_ADDITIONAL = 1;
  const MIN_CURSOR_OFFSET = 0;
  const FIRST_INDEX = 0;
  const INPUT_BORDER_RADIUS_PX = 6;
  const INPUT_PADDING_X_REM = 0.55;
  const INPUT_PADDING_Y_REM = 0.35;
  const INPUT_HEIGHT_REM = 2.2;
  const SUGGESTION_ITEM_GAP_REM = 0.45;
  const SUGGESTION_ITEM_PADDING_X_REM = 0.5;
  const SUGGESTION_ITEM_PADDING_Y_REM = 0.35;
  const SUGGESTION_FONT_SIZE_REM = 0.86;
  const SLICE_START = 0;

  interface ActiveEmoteQuery {
    end: number;
    query: string;
    start: number;
  }

  interface EmoteItem {
    code: string;
    id: string;
    image_url: string;
  }

  interface Props {
    availableEmotes: EmoteItem[];
    disabled?: boolean;
    onSubmit: (text: string) => void;
  }

  const { availableEmotes, disabled = false, onSubmit }: Props = $props();

  // eslint-disable-next-line init-declarations -- Svelte bind:this requires let
  let composerEl = $state<HTMLDivElement | null>(null);
  let suggestionIndex = $state(FIRST_INDEX);
  let suggestionItems = $state<EmoteItem[]>([]);
  let suggestionsOpen = $state(false);
  let text = $state('');

  // Helper functions for DOM operations
  const getTextContentLength = (node: Node): number => {
    const { textContent: nodeText } = node as Text;
    return nodeText === null ? MIN_CURSOR_OFFSET : nodeText.length;
  };

  const getNodeTextContent = (node: Node): string =>
    node.textContent === null ? '' : node.textContent;

  const getClipboardData = (event: ClipboardEvent): string => {
    const { clipboardData: data } = event;
    if (data === null) {
      return '';
    }
    return data.getData('text/plain');
  };

  // Get plain text from composer, preserving structure
  const getComposerText = (): string => {
    if (composerEl === null) {
      return '';
    }
    const { textContent: content } = composerEl;
    return content === null ? '' : content;
  };

  // Get cursor position in text
  const getCursorPosition = (): number => {
    const selection = globalThis.getSelection();
    if (selection === null || composerEl === null || selection.rangeCount === FIRST_INDEX) {
      return text.length;
    }

    const range = selection.getRangeAt(FIRST_INDEX);
    if (!composerEl.contains(range.commonAncestorContainer)) {
      return text.length;
    }

    // Create a range from start of composer to cursor
    const preRange = document.createRange();
    preRange.setStart(composerEl, FIRST_INDEX);
    preRange.setEnd(range.startContainer, range.startOffset);

    // Get length of text before cursor
    const div = document.createElement('div');
    div.append(preRange.cloneContents());
    const { textContent: divText } = div;
    return divText === null ? MIN_CURSOR_OFFSET : divText.length;
  };

  // Set cursor position in text
  const setCursorPosition = (pos: number): void => {
    if (composerEl === null) {
      return;
    }

    const selection = globalThis.getSelection();
    if (selection === null) {
      return;
    }

    // Walk through nodes to find position
    let currentPos = MIN_CURSOR_OFFSET;
    let targetNode: Node | null = null;
    let targetOffset = MIN_CURSOR_OFFSET;

    const walkNodes = (node: Node): boolean => {
      if (node.nodeType === Node.TEXT_NODE) {
        const textContent = getNodeTextContent(node);
        const textLen = textContent.length;
        if (currentPos + textLen >= pos) {
          targetNode = node;
          targetOffset = pos - currentPos;
          return true;
        }
        currentPos += textLen;
      } else if (node.nodeType === Node.ELEMENT_NODE) {
        for (const child of node.childNodes) {
          if (walkNodes(child)) {
            return true;
          }
        }
      }
      return false;
    };

    walkNodes(composerEl);

    if (targetNode !== null) {
      const range = document.createRange();
      const nodeTextLen = getTextContentLength(targetNode);
      range.setStart(targetNode, Math.min(targetOffset, nodeTextLen));
      range.collapse(true);

      selection.removeAllRanges();
      selection.addRange(range);
    }
  };

  // Score an emote based on query match
  const scoreEmote = (code: string, query: string): number => {
    const loweredCode = code.toLowerCase();
    const loweredQuery = query.toLowerCase();
    if (loweredCode === loweredQuery) {
      return SCORE_EXACT_MATCH;
    }
    if (loweredCode.startsWith(loweredQuery)) {
      return SCORE_STARTS_WITH;
    }
    if (loweredCode.includes(loweredQuery)) {
      return SCORE_INCLUDES;
    }
    return SCORE_NO_MATCH;
  };

  // Find active emote query at cursor position
  const findActiveEmoteQuery = (): ActiveEmoteQuery | null => {
    const cursorPos = getCursorPosition();
    const beforeCursor = text.slice(SLICE_START, cursorPos);
    const emotePattern = /(^|\s):([A-Za-z0-9_]{2,})$/;
    const match = beforeCursor.match(emotePattern);
    if (match === null) {
      return null;
    }
    const [, , query] = match;
    return {
      end: beforeCursor.length,
      query,
      start: beforeCursor.length - query.length - CURSOR_POS_ADDITIONAL,
    };
  };

  // Close suggestions dropdown
  const closeSuggestions = (): void => {
    suggestionsOpen = false;
    suggestionItems = [];
    suggestionIndex = FIRST_INDEX;
  };

  // Refresh emote suggestions based on current query
  const refreshSuggestions = (): void => {
    const query = findActiveEmoteQuery();
    if (query === null) {
      closeSuggestions();
      return;
    }

    const search = query.query.toLowerCase();
    const ranked = availableEmotes
      .map((item) => ({ item, score: scoreEmote(item.code, search) }))
      .filter((entry) => entry.score < SCORE_NO_MATCH)
      .toSorted((left, right) => {
        if (left.score !== right.score) {
          return left.score - right.score;
        }
        return left.item.code.toLowerCase().localeCompare(right.item.code.toLowerCase());
      })
      .slice(SLICE_START, MAX_SUGGESTIONS)
      .map((entry) => entry.item);

    const EMPTY_LENGTH = 0;
    if (ranked.length === EMPTY_LENGTH) {
      closeSuggestions();
      return;
    }

    suggestionsOpen = true;
    suggestionItems = ranked;
    suggestionIndex = Math.min(suggestionIndex, ranked.length - CURSOR_POS_ADDITIONAL);
  };

  // Insert an emote at a specific text range
  const insertEmoteAtRange = (code: string, range: ActiveEmoteQuery): void => {
    const safeCode = code.trim();
    if (safeCode === '') {
      return;
    }

    const before = text.slice(SLICE_START, range.start);
    const after = text.slice(range.end);
    const newText = `${before}${safeCode} ${after}`;

    // Calculate new cursor position
    const newCursorPos = range.start + safeCode.length + CURSOR_POS_ADDITIONAL;

    text = newText;
    if (composerEl !== null) {
      composerEl.textContent = newText;
      setCursorPosition(Math.min(newCursorPos, newText.length));
    }

    closeSuggestions();
  };

  // Submit the current text
  const submit = (): void => {
    const trimmed = text.trim();
    if (trimmed === '' || disabled) {
      return;
    }
    onSubmit(trimmed);
    text = '';
    if (composerEl !== null) {
      composerEl.textContent = '';
    }
    closeSuggestions();
  };

  // Handle input events
  const handleInput = (): void => {
    const newText = getComposerText();
    // Remove line breaks and limit length
    if (newText.includes('\n') || newText.includes('\r')) {
      const lineBreakPattern = /[\r\n]+/g;
      text = newText.replaceAll(lineBreakPattern, ' ').slice(FIRST_INDEX, MAX_TEXT_LENGTH);
      // Re-render with normalized text
      if (composerEl !== null) {
        const cursorPos = getCursorPosition();
        composerEl.textContent = text;
        setCursorPosition(Math.min(cursorPos, text.length));
      }
    } else {
      text = newText.slice(FIRST_INDEX, MAX_TEXT_LENGTH);
    }
    refreshSuggestions();
  };

  // Handle keydown events
  const handleKeydown = (event: KeyboardEvent): void => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      if (suggestionsOpen && suggestionItems.length > FIRST_INDEX) {
        const selected = suggestionItems[suggestionIndex];
        const query = findActiveEmoteQuery();
        if (selected !== undefined && query !== null) {
          insertEmoteAtRange(selected.code, query);
        }
        closeSuggestions();
      } else {
        submit();
      }
      return;
    }

    if (!suggestionsOpen || suggestionItems.length === FIRST_INDEX) {
      if (event.key === 'Escape') {
        event.preventDefault();
      }
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      suggestionIndex = (suggestionIndex + CURSOR_POS_ADDITIONAL) % suggestionItems.length;
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      const prevIndex = suggestionIndex - CURSOR_POS_ADDITIONAL + suggestionItems.length;
      suggestionIndex = prevIndex % suggestionItems.length;
      return;
    }

    if (event.key === 'Tab' || (event.key === 'Enter' && event.shiftKey)) {
      event.preventDefault();
      const selected = suggestionItems[suggestionIndex];
      const query = findActiveEmoteQuery();
      if (selected !== undefined && query !== null) {
        insertEmoteAtRange(selected.code, query);
      }
      closeSuggestions();
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      closeSuggestions();
    }
  };

  // Handle paste events
  const handlePaste = (event: ClipboardEvent): void => {
    event.preventDefault();
    const pasted = getClipboardData(event);
    const lineBreakPattern = /[\r\n]+/g;
    const cleaned = pasted.replaceAll(lineBreakPattern, ' ');
    if (cleaned === '') {
      return;
    }

    const cursorPos = getCursorPosition();
    const before = text.slice(FIRST_INDEX, cursorPos);
    const after = text.slice(cursorPos);
    const newText = (before + cleaned + after).slice(FIRST_INDEX, MAX_TEXT_LENGTH);

    text = newText;
    if (composerEl !== null) {
      composerEl.textContent = newText;
      setCursorPosition(cursorPos + cleaned.length);
    }

    refreshSuggestions();
  };

  // Handle suggestion item click
  const handleSuggestionClick = (item: EmoteItem, event: MouseEvent): void => {
    event.preventDefault();
    const query = findActiveEmoteQuery();
    if (query !== null) {
      insertEmoteAtRange(item.code, query);
    }
    closeSuggestions();
  };

  // Public method to insert an emote by code from outside (e.g., emote picker)
  export const insertEmote = (code: string): void => {
    if (composerEl === null || disabled) {
      return;
    }

    const cursorPos = getCursorPosition();
    const before = text.slice(FIRST_INDEX, cursorPos);
    const after = text.slice(cursorPos);

    // Add space before emote if not at start and previous char isn't space
    const prefix = before.length > FIRST_INDEX && !before.endsWith(' ') ? ' ' : '';
    // Add space after emote
    const suffix = ' ';

    const safeCode = code.trim();
    if (safeCode === '') {
      return;
    }

    const newText = `${before}${prefix}${safeCode}${suffix}${after}`.slice(FIRST_INDEX, MAX_TEXT_LENGTH);
    const newCursorPos = before.length + prefix.length + safeCode.length + suffix.length;

    text = newText;
    composerEl.textContent = newText;
    setCursorPosition(Math.min(newCursorPos, newText.length));
    closeSuggestions();
  };
</script>

<div class="composer-container">
  <div
    bind:this={composerEl}
    class="composer-input"
    class:is-disabled={disabled}
    contenteditable={!disabled}
    role="textbox"
    tabindex={disabled ? TAB_INDEX_DISABLED : TAB_INDEX_ENABLED}
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
