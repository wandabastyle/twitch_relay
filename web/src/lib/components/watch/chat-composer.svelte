<script lang="ts">
  import {
    findActiveEmoteQuery,
    insertCodeAtCursor,
    normalizeSingleLine,
    rankSuggestions,
    replaceRangeWithEmote,
    trimToMaxLength,
    type EmoteItem,
  } from './chat-composer-helpers.svelte';
  const MAX_TEXT_LENGTH = 500;
  const MAX_SUGGESTIONS = 10;
  const ZERO = 0;
  const ONE = 1;
  const TAB_INDEX_DISABLED = -1;
  const TAB_INDEX_ENABLED = 0;

  interface Props {
    availableEmotes: EmoteItem[];
    disabled?: boolean;
    onSubmit: (text: string) => void;
  }
  const { availableEmotes, disabled = false, onSubmit }: Props = $props();
  const composer = $state({ el: null as HTMLDivElement | null });
  let suggestionIndex = $state(ZERO);
  let suggestionItems = $state<EmoteItem[]>([]);
  let suggestionsOpen = $state(false);
  let text = $state('');
  const readComposerText = (): string => composer.el?.textContent ?? '';
  const setComposerText = (value: string): void => {
    text = value;
    if (composer.el !== null) {
      composer.el.textContent = value;
    }
  };
  const getSelectionRange = (): Range | null => {
    const selection = globalThis.getSelection();
    if (selection === null || composer.el === null || selection.rangeCount === ZERO) {
      return null;
    }
    const range = selection.getRangeAt(ZERO);
    return composer.el.contains(range.commonAncestorContainer) ? range : null;
  };
  const getRangeTextLength = (range: Range): number => {
    if (composer.el === null) {
      return text.length;
    }
    const preRange = document.createRange();
    preRange.setStart(composer.el, ZERO);
    preRange.setEnd(range.startContainer, range.startOffset);
    const div = document.createElement('div');
    div.append(preRange.cloneContents());
    return div.textContent?.length ?? ZERO;
  };
  const getCursorPosition = (): number => getSelectionRange() === null ? text.length : getRangeTextLength(getSelectionRange() as Range);
  const walkToCursorTarget = (
    node: Node,
    position: number,
    state: { currentPos: number; targetNode: Node | null; targetOffset: number },
  ): boolean => {
    if (node.nodeType === Node.TEXT_NODE) {
      const len = node.textContent?.length ?? ZERO;
      if (state.currentPos + len >= position) {
        state.targetNode = node;
        state.targetOffset = position - state.currentPos;
        return true;
      }
      state.currentPos += len;
    }
    return [...node.childNodes].some((child) => walkToCursorTarget(child, position, state));
  };
  const applyCursorSelection = (selection: Selection, node: Node, offset: number): void => {
    const range = document.createRange();
    const maxOffset = node.textContent?.length ?? ZERO;
    range.setStart(node, Math.min(offset, maxOffset));
    range.collapse(true);
    selection.removeAllRanges();
    selection.addRange(range);
  };
  const setCursorPosition = (position: number): void => {
    if (composer.el === null) {
      return;
    }
    const selection = globalThis.getSelection();
    if (selection === null) {
      return;
    }
    const state = { currentPos: ZERO, targetNode: null as Node | null, targetOffset: ZERO };
    walkToCursorTarget(composer.el, position, state);
    if (state.targetNode === null) {
      return;
    }
    applyCursorSelection(selection, state.targetNode, state.targetOffset);
  };
  const closeSuggestions = (): void => {
    suggestionsOpen = false;
    suggestionItems = [];
    suggestionIndex = ZERO;
  };
  const findCurrentQuery = (): ReturnType<typeof findActiveEmoteQuery> =>
    findActiveEmoteQuery(text, getCursorPosition());
  const applySuggestion = (code: string): void => {
    const query = findCurrentQuery();
    if (query === null) {
      closeSuggestions();
      return;
    }
    const next = replaceRangeWithEmote(text, query, code);
    setComposerText(next.text);
    setCursorPosition(Math.min(next.cursor, next.text.length));
    closeSuggestions();
  };
  const selectSuggestion = (item: EmoteItem): void => {
    const safeCode = item.code.trim();
    if (safeCode === '') {
      closeSuggestions();
      return;
    }
    applySuggestion(safeCode);
  };
  const refreshSuggestions = (): void => {
    const query = findCurrentQuery();
    const ranked = query === null ? [] : rankSuggestions(availableEmotes, query.query, MAX_SUGGESTIONS);
    if (ranked.length === ZERO) {
      closeSuggestions();
      return;
    }
    suggestionsOpen = true;
    suggestionItems = ranked;
    suggestionIndex = Math.min(suggestionIndex, ranked.length - ONE);
  };
  const submit = (): void => {
    const trimmed = text.trim();
    if (trimmed === '' || disabled) {
      return;
    }
    onSubmit(trimmed);
    setComposerText('');
    closeSuggestions();
  };
  const handleInput = (): void => {
    const cursorPos = getCursorPosition();
    const rawText = readComposerText();
    const nextText = trimToMaxLength(normalizeSingleLine(rawText), MAX_TEXT_LENGTH);
    if (nextText === rawText) {
      text = nextText;
      refreshSuggestions();
      return;
    }
    setComposerText(nextText);
    setCursorPosition(Math.min(cursorPos, nextText.length));
    refreshSuggestions();
  };
  const hasActiveSelection = (): boolean => suggestionsOpen && suggestionItems.length > ZERO;
  const selectCurrentSuggestion = (): void => {
    const selected = suggestionItems[suggestionIndex];
    if (selected !== undefined) {
      selectSuggestion(selected);
    }
  };
  const moveSelection = (delta: number): void => {
    suggestionIndex = (suggestionIndex + delta + suggestionItems.length) % suggestionItems.length;
  };
  const handleEnterKey = (event: KeyboardEvent, activeSelection: boolean): boolean => {
    if (event.key !== 'Enter' || event.shiftKey) {
      return false;
    }
    event.preventDefault();
    if (activeSelection) {
      selectCurrentSuggestion();
    } else {
      submit();
    }
    return true;
  };
  const handleInactiveSelectionKey = (event: KeyboardEvent): boolean => {
    if (event.key !== 'Escape') {
      return false;
    }
    event.preventDefault();
    return true;
  };
  const handleSelectionKey = (event: KeyboardEvent): boolean => {
    if (event.key === 'Tab' || (event.key === 'Enter' && event.shiftKey)) {
      event.preventDefault();
      selectCurrentSuggestion();
      return true;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      moveSelection(event.key === 'ArrowDown' ? ONE : -ONE);
      return true;
    }
    return false;
  };
  const handleEscapeKey = (event: KeyboardEvent): void => {
    if (event.key === 'Escape') {
      event.preventDefault();
      closeSuggestions();
    }
  };
  const handleKeydown = (event: KeyboardEvent): void => {
    const activeSelection = hasActiveSelection();
    if (handleEnterKey(event, activeSelection)) {
      return;
    }
    if (!activeSelection) {
      handleInactiveSelectionKey(event);
      return;
    }
    if (handleSelectionKey(event)) {
      return;
    }
    handleEscapeKey(event);
  };
  const handlePaste = (event: ClipboardEvent): void => {
    event.preventDefault();
    const pasted = event.clipboardData?.getData('text/plain') ?? '';
    const cleaned = normalizeSingleLine(pasted);
    if (cleaned === '') {
      return;
    }
    const cursorPos = getCursorPosition();
    const nextText = trimToMaxLength(
      text.slice(ZERO, cursorPos) + cleaned + text.slice(cursorPos),
      MAX_TEXT_LENGTH,
    );
    setComposerText(nextText);
    setCursorPosition(Math.min(cursorPos + cleaned.length, nextText.length));
    refreshSuggestions();
  };
  const handleSuggestionClick = (item: EmoteItem, event: MouseEvent): void => {
    event.preventDefault();
    selectSuggestion(item);
  };
  export const insertEmote = (code: string): void => {
    if (composer.el === null || disabled) {
      return;
    }
    const safeCode = code.trim();
    if (safeCode === '') {
      return;
    }
    const next = insertCodeAtCursor({
      code: safeCode,
      cursorPos: getCursorPosition(),
      maxLength: MAX_TEXT_LENGTH,
      text,
    });
    setComposerText(next.text);
    setCursorPosition(next.cursor);
    closeSuggestions();
  };
</script>
<div class="composer-container">
  <div
    bind:this={composer.el}
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
<style src="./chat-composer.css"></style>
