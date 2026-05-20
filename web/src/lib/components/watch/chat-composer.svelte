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
  const HALF = 2;
  const TAB_INDEX_DISABLED = -1;
  const TAB_INDEX_ENABLED = 0;

  interface EmoteChip {
    code: string;
    image_url: string;
    position: number;
  }

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
  let emoteChips = $state<EmoteChip[]>([]);

  const getEmoteImageUrl = (code: string): string | null => {
    const emote = availableEmotes.find((emoteItem) => emoteItem.code === code);
    return emote?.image_url ?? null;
  };

  const createEmoteImageElement = (code: string, imageUrl: string): HTMLImageElement => {
    const img = document.createElement('img');
    img.className = 'ui-chat-composer-emote';
    img.dataset.code = code;
    img.dataset.imageUrl = imageUrl;
    img.src = imageUrl;
    img.alt = code;
    img.contentEditable = 'false';
    img.draggable = false;
    return img;
  };

  const renderComposerContent = (textValue: string, chips: EmoteChip[]): void => {
    if (composer.el === null) {
      return;
    }

    // Sort chips by position
    const sortedChips = [...chips].toSorted((first, second) => first.position - second.position);

    // Clear existing content
    composer.el.textContent = '';

    let currentPos = 0;
    for (const chip of sortedChips) {
      // Add text before this chip
      const textBefore = textValue.slice(currentPos, chip.position);
      if (textBefore.length > ZERO) {
        composer.el.append(document.createTextNode(textBefore));
      }

      // Add the emote image
      const img = createEmoteImageElement(chip.code, chip.image_url);
      composer.el.append(img);

      // Move past the emote code in the text
      currentPos = chip.position + chip.code.length;
    }

    // Add remaining text
    const textAfter = textValue.slice(currentPos);
    if (textAfter.length > ZERO) {
      composer.el.append(document.createTextNode(textAfter));
    }
  };

  interface ComposerModel {
    text: string;
    chips: EmoteChip[];
  }

  const readComposerModel = (): ComposerModel => {
    if (composer.el === null) {
      return { chips: [], text: '' };
    }

    let resultText = '';
    const chips: EmoteChip[] = [];

    for (const node of composer.el.childNodes) {
      if (node.nodeType === Node.TEXT_NODE) {
        resultText += node.textContent ?? '';
      } else if (node.nodeType === Node.ELEMENT_NODE && node instanceof HTMLImageElement) {
        const { code } = node.dataset;
        const { imageUrl } = node.dataset;
        if (code !== undefined && code !== '' && imageUrl !== undefined && imageUrl !== '') {
          resultText += code;
          chips.push({
            code,
            image_url: imageUrl,
            position: resultText.length - code.length,
          });
        }
      }
    }

    return { chips, text: resultText };
  };

  const readComposerText = (): string => readComposerModel().text;

  const setComposerText = (value: string, chips: EmoteChip[] = []): void => {
    text = value;
    emoteChips = chips;
    renderComposerContent(value, chips);
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

    // Walk nodes and count text length
    // For emote images, count data-code.length instead of 0 (textContent returns 0 for images)
    let length = ZERO;
    // Use numeric addition instead of bitwise OR to satisfy lint rules
    const nodeFilter = Number(NodeFilter.SHOW_ELEMENT) + Number(NodeFilter.SHOW_TEXT);
    const walker = document.createTreeWalker(div, nodeFilter, null);
    while (walker.nextNode()) {
      const node = walker.currentNode;
      if (node.nodeType === Node.TEXT_NODE) {
        length += node.textContent?.length ?? ZERO;
      } else if (node.nodeType === Node.ELEMENT_NODE && node instanceof HTMLImageElement) {
        const { code } = node.dataset;
        if (code !== undefined && code !== '') {
          length += code.length;
        }
      }
    }
    return length;
  };
  const getCursorPosition = (): number => getSelectionRange() === null ? text.length : getRangeTextLength(getSelectionRange() as Range);
  const walkToCursorTarget = (
    node: Node,
    position: number,
    state: { currentPos: number; targetNode: Node | null; targetOffset: number },
  ): boolean => {
    // Handle text nodes
    if (node.nodeType === Node.TEXT_NODE) {
      const len = node.textContent?.length ?? ZERO;
      if (state.currentPos + len >= position) {
        state.targetNode = node;
        state.targetOffset = position - state.currentPos;
        return true;
      }
      state.currentPos += len;
      return false;
    }

    // Handle emote image nodes - count data-code length
    if (node.nodeType === Node.ELEMENT_NODE && node instanceof HTMLImageElement) {
      const { code } = node.dataset;
      if (code !== undefined && code !== '') {
        const codeLength = code.length;
        // Check if cursor should be inside this image's position range
        if (state.currentPos + codeLength >= position) {
          // Cursor is within this emote's text position
          // Place cursor in the parent element before or after the image
          const parent = node.parentNode;
          if (parent !== null) {
            const nodeIndex = [...parent.childNodes].indexOf(node);
            const offsetInEmote = position - state.currentPos;
            // Place cursor in the parent element before or after the image
            state.targetNode = parent;
            // Determine if cursor should be before or after the image
            const isBeforeMidpoint = offsetInEmote <= codeLength / HALF;
            state.targetOffset = isBeforeMidpoint ? nodeIndex : nodeIndex + ONE;
          }
          return true;
        }
        state.currentPos += codeLength;
      }
      return false;
    }

    // Walk child nodes for other element types
    return [...node.childNodes].some((child) => walkToCursorTarget(child, position, state));
  };
  const applyCursorSelection = (selection: Selection, node: Node, offset: number): void => {
    const range = document.createRange();

    // Handle element nodes (e.g., composer.el itself when placing cursor before/after images)
    if (node.nodeType === Node.ELEMENT_NODE) {
      const element = node as Element;
      const childCount = element.childNodes.length;
      const safeOffset = Math.min(offset, childCount);
      range.setStart(node, safeOffset);
      range.collapse(true);
      selection.removeAllRanges();
      selection.addRange(range);
      return;
    }

    // Handle text nodes
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
    const imageUrl = getEmoteImageUrl(code);

    if (imageUrl === null) {
      // Emote not found, just set text without chips
      setComposerText(next.text);
      setCursorPosition(Math.min(next.cursor, next.text.length));
      closeSuggestions();
      return;
    }

    // Calculate the new emote position after insertion
    const newEmotePosition = query.start;

    // Create new chips array with updated positions
    const newChips: EmoteChip[] = [];
    const lengthDiff = next.text.length - text.length;

    for (const chip of emoteChips) {
      if (chip.position < query.start) {
        // Chip is before the insertion point, keep as is
        newChips.push(chip);
      } else {
        // Chip is after the insertion point, adjust position
        newChips.push({ ...chip, position: chip.position + lengthDiff });
      }
    }

    // Add the new emote chip
    newChips.push({
      code,
      image_url: imageUrl,
      position: newEmotePosition,
    });

    setComposerText(next.text, newChips);
    setCursorPosition(next.cursor);
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
    setComposerText('', []);
    closeSuggestions();
  };
  const handleInput = (): void => {
    const cursorPos = getCursorPosition();
    const model = readComposerModel();
    const rawText = model.text;
    const nextText = trimToMaxLength(normalizeSingleLine(rawText), MAX_TEXT_LENGTH);
    if (nextText === rawText) {
      // Text hasn't changed (just formatting), update both text and chips from DOM
      text = nextText;
      emoteChips = model.chips;
      refreshSuggestions();
      return;
    }
    // Text changed significantly, we need to re-render and lose cursor position
    setComposerText(nextText, model.chips);
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

    const cursorPos = getCursorPosition();
    const next = insertCodeAtCursor({
      code: safeCode,
      cursorPos,
      maxLength: MAX_TEXT_LENGTH,
      text,
    });

    const imageUrl = getEmoteImageUrl(safeCode);
    if (imageUrl === null) {
      // Emote not found, just set text without chips
      setComposerText(next.text);
      setCursorPosition(next.cursor);
      closeSuggestions();
      return;
    }

    // Calculate the new emote position
    // The new emote is inserted at cursorPos (with a space before if not at start)
    const SPACE_OFFSET = ONE;
    const newEmotePosition = cursorPos === text.length && text.length > ZERO ? cursorPos + SPACE_OFFSET : cursorPos;

    // Create new chips array with updated positions
    const newChips: EmoteChip[] = [];
    const lengthDiff = next.text.length - text.length;

    for (const chip of emoteChips) {
      if (chip.position < cursorPos) {
        // Chip is before the insertion point, keep as is
        newChips.push(chip);
      } else {
        // Chip is after the insertion point, adjust position
        newChips.push({ ...chip, position: chip.position + lengthDiff });
      }
    }

    // Add the new emote chip
    newChips.push({
      code: safeCode,
      image_url: imageUrl,
      position: newEmotePosition,
    });

    setComposerText(next.text, newChips);
    setCursorPosition(next.cursor);
    closeSuggestions();
  };
</script>
<div class="ui-chat-composer">
  <div
    bind:this={composer.el}
    class="ui-chat-composer-input"
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
    <div class="ui-chat-suggestions ui-hide-scrollbar">
      {#each suggestionItems as item, index (item.id)}
        <button
          type="button"
          class="ui-chat-suggestion-item"
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
