import { useCallback, useRef, useState } from 'react';
import type { EmoteItem } from '../../api-client';
import {
  findActiveEmoteQuery,
  insertCodeAtCursor,
  normalizeSingleLine,
  rankSuggestions,
  replaceRangeWithEmote,
  trimToMaxLength,
  type ActiveEmoteQuery,
} from '../../lib/components/watch/chat-composer-helpers.svelte';

const MAX_TEXT_LENGTH = 500;
const MAX_SUGGESTIONS = 10;
const ZERO = 0;
const ONE = 1;
const HALF = 2;
const NBSP = '\u00A0';

interface EmoteChip {
  code: string;
  image_url: string;
  position: number;
}

export interface UseChatComposerReturn {
  text: string;
  emoteChips: EmoteChip[];
  suggestionsOpen: boolean;
  suggestionItems: EmoteItem[];
  suggestionIndex: number;
  previewOpen: boolean;
  previewUrl: string;
  previewPosition: { left: number; top: number };
  composerRef: React.RefObject<HTMLDivElement | null>;
  previewTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>;
  handleInput: () => void;
  handlePaste: (event: React.ClipboardEvent) => void;
  handleKeydown: (event: React.KeyboardEvent) => void;
  handleSuggestionClick: (item: EmoteItem) => void;
  insertEmote: (code: string) => void;
  submit: () => void;
  closeSuggestions: () => void;
  clearPreview: () => void;
  createEmoteImageElement: (code: string, imageUrl: string) => HTMLSpanElement;
  renderComposerContent: (textValue: string, chips: EmoteChip[]) => void;
  readComposerModel: () => { text: string; chips: EmoteChip[] };
  setComposerText: (value: string, chips?: EmoteChip[]) => void;
}

export interface UseChatComposerOptions {
  availableEmotes: EmoteItem[];
  disabled?: boolean;
  onSubmit: (text: string) => void;
}

export function useChatComposer(options: UseChatComposerOptions): UseChatComposerReturn {
  const { availableEmotes, disabled = false, onSubmit } = options;

  const composerRef = useRef<HTMLDivElement>(null);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [text, setText] = useState('');
  const [emoteChips, setEmoteChips] = useState<EmoteChip[]>([]);
  const [suggestionsOpen, setSuggestionsOpen] = useState(false);
  const [suggestionItems, setSuggestionItems] = useState<EmoteItem[]>([]);
  const [suggestionIndex, setSuggestionIndex] = useState(ZERO);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewUrl, setPreviewUrl] = useState('');
  const [previewPosition, setPreviewPosition] = useState({ left: ZERO, top: ZERO });

  const PREVIEW_DELAY_MS = 350;
  const PREVIEW_SIZE = 112;
  const PREVIEW_OFFSET = 8;
  const CENTER_DIVISOR = 2;

  const getEmoteImageUrl = useCallback(
    (code: string): string | null => {
      const emote = availableEmotes.find((emoteItem) => emoteItem.code === code);
      return emote?.image_url ?? null;
    },
    [availableEmotes],
  );

  const createEmoteImageElement = useCallback((code: string, imageUrl: string): HTMLSpanElement => {
    const wrapper = document.createElement('span');
    wrapper.className = 'ui-chat-composer-emote-wrap';
    wrapper.contentEditable = 'false';

    const img = document.createElement('img');
    img.className = 'ui-chat-composer-emote';
    img.dataset.code = code;
    img.dataset.imageUrl = imageUrl;
    img.src = imageUrl;
    img.alt = code;
    img.draggable = false;

    // Add hover preview handlers
    wrapper.addEventListener('mouseenter', () => {
      // Clear any existing timer
      if (previewTimerRef.current !== null) {
        clearTimeout(previewTimerRef.current);
      }
      // Set new timer
      previewTimerRef.current = setTimeout(() => {
        const rect = wrapper.getBoundingClientRect();
        setPreviewUrl(imageUrl);
        setPreviewPosition({
          left: rect.left + rect.width / CENTER_DIVISOR - PREVIEW_SIZE / CENTER_DIVISOR,
          top: rect.top - PREVIEW_SIZE - PREVIEW_OFFSET,
        });
        setPreviewOpen(true);
      }, PREVIEW_DELAY_MS);
    });

    wrapper.addEventListener('mouseleave', () => {
      if (previewTimerRef.current !== null) {
        clearTimeout(previewTimerRef.current);
        previewTimerRef.current = null;
      }
      setPreviewOpen(false);
    });

    wrapper.append(img);
    return wrapper;
  }, []);

  const renderComposerContent = useCallback(
    (textValue: string, chips: EmoteChip[]): void => {
      if (composerRef.current === null) {
        return;
      }

      // Sort chips by position
      const sortedChips = [...chips].sort((first, second) => first.position - second.position);

      // Clear existing content
      composerRef.current.textContent = '';

      let currentPos = 0;
      for (const chip of sortedChips) {
        // Add text before this chip
        const textBefore = textValue.slice(currentPos, chip.position);
        if (textBefore.length > ZERO) {
          // Use NBSP to make trailing spaces visible in contenteditable
          composerRef.current.append(document.createTextNode(textBefore.replaceAll(' ', NBSP)));
        }

        // Add the emote image
        const img = createEmoteImageElement(chip.code, chip.image_url);
        composerRef.current.append(img);

        // Move past the emote code in the text
        currentPos = chip.position + chip.code.length;
      }

      // Add remaining text
      const textAfter = textValue.slice(currentPos);
      if (textAfter.length > ZERO) {
        // Use NBSP to make trailing spaces visible in contenteditable
        composerRef.current.append(document.createTextNode(textAfter.replaceAll(' ', NBSP)));
      }
    },
    [createEmoteImageElement],
  );

  const readComposerModel = useCallback((): { text: string; chips: EmoteChip[] } => {
    if (composerRef.current === null) {
      return { chips: [], text: '' };
    }

    let resultText = '';
    const chips: EmoteChip[] = [];

    for (const node of composerRef.current.childNodes) {
      if (node.nodeType === Node.TEXT_NODE) {
        // Convert NBSP back to regular spaces for canonical text
        const textContent = node.textContent ?? '';
        resultText += textContent.replaceAll(NBSP, ' ');
      } else if (
        node.nodeType === Node.ELEMENT_NODE &&
        node instanceof HTMLSpanElement &&
        node.classList.contains('ui-chat-composer-emote-wrap')
      ) {
        const img = node.querySelector('img.ui-chat-composer-emote') as HTMLImageElement | null;
        if (img !== null) {
          const { code } = img.dataset;
          const { imageUrl } = img.dataset;
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
    }

    return { chips, text: resultText };
  }, []);

  const setComposerText = useCallback(
    (value: string, chips: EmoteChip[] = []): void => {
      setText(value);
      setEmoteChips(chips);
      renderComposerContent(value, chips);
    },
    [renderComposerContent],
  );

  const getSelectionRange = useCallback((): Range | null => {
    const selection = globalThis.getSelection();
    if (selection === null || composerRef.current === null || selection.rangeCount === ZERO) {
      return null;
    }
    const range = selection.getRangeAt(ZERO);
    return composerRef.current.contains(range.commonAncestorContainer) ? range : null;
  }, []);

  const getRangeTextLength = useCallback(
    (range: Range): number => {
      if (composerRef.current === null) {
        return text.length;
      }
      const preRange = document.createRange();
      preRange.setStart(composerRef.current, ZERO);
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
    },
    [text.length],
  );

  const getCursorPosition = useCallback((): number => {
    return getSelectionRange() === null
      ? text.length
      : getRangeTextLength(getSelectionRange() as Range);
  }, [getSelectionRange, getRangeTextLength, text.length]);

  const walkToCursorTarget = useCallback(
    (
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

      // Handle emote wrapper nodes - count data-code length from child img
      if (
        node.nodeType === Node.ELEMENT_NODE &&
        node instanceof HTMLSpanElement &&
        node.classList.contains('ui-chat-composer-emote-wrap')
      ) {
        const img = node.querySelector('img.ui-chat-composer-emote') as HTMLImageElement | null;
        const { code } = img?.dataset ?? {};
        if (code === undefined || code === '') {
          return false;
        }
        const codeLength = code.length;
        // Check if cursor should be inside this emote's position range
        if (state.currentPos + codeLength >= position) {
          // Cursor is within this emote's text position
          // Place cursor in the parent element before or after the wrapper
          const parent = node.parentNode;
          const nodeIndex = parent === null ? ZERO : [...parent.childNodes].indexOf(node);
          const offsetInEmote = position - state.currentPos;
          // Determine if cursor should be before or after the wrapper
          const isBeforeMidpoint = offsetInEmote <= codeLength / HALF;
          state.targetNode = parent;
          state.targetOffset = isBeforeMidpoint ? nodeIndex : nodeIndex + ONE;
          return true;
        }
        state.currentPos += codeLength;
        return false;
      }

      // Walk child nodes for other element types
      return [...node.childNodes].some((child) => walkToCursorTarget(child, position, state));
    },
    [],
  );

  const applyCursorSelection = useCallback(
    (selection: Selection, node: Node, offset: number): void => {
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
    },
    [],
  );

  const setCursorPosition = useCallback(
    (position: number): void => {
      if (composerRef.current === null) {
        return;
      }
      const selection = globalThis.getSelection();
      if (selection === null) {
        return;
      }
      const state = { currentPos: ZERO, targetNode: null as Node | null, targetOffset: ZERO };
      walkToCursorTarget(composerRef.current, position, state);
      if (state.targetNode === null) {
        return;
      }
      applyCursorSelection(selection, state.targetNode, state.targetOffset);
    },
    [walkToCursorTarget, applyCursorSelection],
  );

  const closeSuggestions = useCallback((): void => {
    setSuggestionsOpen(false);
    setSuggestionItems([]);
    setSuggestionIndex(ZERO);
  }, []);

  const findCurrentQuery = useCallback((): ActiveEmoteQuery | null => {
    return findActiveEmoteQuery(text, getCursorPosition());
  }, [text, getCursorPosition]);

  const applySuggestion = useCallback(
    (code: string): void => {
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
    },
    [
      findCurrentQuery,
      closeSuggestions,
      text,
      getEmoteImageUrl,
      setComposerText,
      setCursorPosition,
      emoteChips,
    ],
  );

  const selectSuggestion = useCallback(
    (item: EmoteItem): void => {
      const safeCode = item.code.trim();
      if (safeCode === '') {
        closeSuggestions();
        return;
      }
      applySuggestion(safeCode);
    },
    [applySuggestion, closeSuggestions],
  );

  const refreshSuggestions = useCallback((): void => {
    const query = findCurrentQuery();
    // Convert EmoteItem from api-client to match what rankSuggestions expects
    const emotesForRanking = availableEmotes.map((emote) => ({
      code: emote.code,
      id: emote.id,
      image_url: emote.image_url,
    }));
    const ranked =
      query === null ? [] : rankSuggestions(emotesForRanking, query.query, MAX_SUGGESTIONS);
    if (ranked.length === ZERO) {
      closeSuggestions();
      return;
    }
    // Map back to full EmoteItem
    const rankedEmotes: EmoteItem[] = [];
    for (const rankedItem of ranked) {
      const fullEmote = availableEmotes.find((emote) => emote.code === rankedItem.code);
      if (fullEmote) {
        rankedEmotes.push(fullEmote);
      }
    }
    setSuggestionsOpen(true);
    setSuggestionItems(rankedEmotes);
    setSuggestionIndex((prev) => Math.min(prev, rankedEmotes.length - ONE));
  }, [findCurrentQuery, availableEmotes, closeSuggestions]);

  const submit = useCallback((): void => {
    const trimmed = text.trim();
    if (trimmed === '' || disabled) {
      return;
    }
    onSubmit(trimmed);
    setComposerText('', []);
    closeSuggestions();
  }, [text, disabled, onSubmit, setComposerText, closeSuggestions]);

  const handleInput = useCallback((): void => {
    const cursorPos = getCursorPosition();
    const model = readComposerModel();
    const rawText = model.text;
    const nextText = trimToMaxLength(normalizeSingleLine(rawText), MAX_TEXT_LENGTH);
    if (nextText === rawText) {
      // Text hasn't changed (just formatting), update both text and chips from DOM
      setText(nextText);
      setEmoteChips(model.chips);
      refreshSuggestions();
      return;
    }
    // Text changed significantly, we need to re-render and lose cursor position
    setComposerText(nextText, model.chips);
    setCursorPosition(Math.min(cursorPos, nextText.length));
    refreshSuggestions();
  }, [
    getCursorPosition,
    readComposerModel,
    setComposerText,
    setCursorPosition,
    refreshSuggestions,
  ]);

  const hasActiveSelection = useCallback((): boolean => {
    return suggestionsOpen && suggestionItems.length > ZERO;
  }, [suggestionsOpen, suggestionItems.length]);

  const selectCurrentSuggestion = useCallback((): void => {
    const selected = suggestionItems[suggestionIndex];
    if (selected !== undefined) {
      selectSuggestion(selected);
    }
  }, [suggestionItems, suggestionIndex, selectSuggestion]);

  const moveSelection = useCallback(
    (delta: number): void => {
      setSuggestionIndex(
        (prev) => (prev + delta + suggestionItems.length) % suggestionItems.length,
      );
    },
    [suggestionItems.length],
  );

  const handleEnterKey = useCallback(
    (event: React.KeyboardEvent, activeSelection: boolean): boolean => {
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
    },
    [selectCurrentSuggestion, submit],
  );

  const handleInactiveSelectionKey = useCallback((event: React.KeyboardEvent): boolean => {
    if (event.key !== 'Escape') {
      return false;
    }
    event.preventDefault();
    return true;
  }, []);

  const handleSelectionKey = useCallback(
    (event: React.KeyboardEvent): boolean => {
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
    },
    [selectCurrentSuggestion, moveSelection],
  );

  const handleEscapeKey = useCallback(
    (event: React.KeyboardEvent): void => {
      if (event.key === 'Escape') {
        event.preventDefault();
        closeSuggestions();
      }
    },
    [closeSuggestions],
  );

  const handleKeydown = useCallback(
    (event: React.KeyboardEvent): void => {
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
    },
    [
      hasActiveSelection,
      handleEnterKey,
      handleInactiveSelectionKey,
      handleSelectionKey,
      handleEscapeKey,
    ],
  );

  const handlePaste = useCallback(
    (event: React.ClipboardEvent): void => {
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
    },
    [getCursorPosition, text, setComposerText, setCursorPosition, refreshSuggestions],
  );

  const handleSuggestionClick = useCallback(
    (item: EmoteItem): void => {
      selectSuggestion(item);
    },
    [selectSuggestion],
  );

  const insertEmote = useCallback(
    (code: string): void => {
      if (composerRef.current === null || disabled) {
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

      // Calculate the new emote position using same logic as insertCodeAtCursor
      // The emote is inserted at cursorPos, with a space prefix if not at start and no space before
      const before = text.slice(ZERO, cursorPos);
      const prefixLength = before.length > ZERO && !before.endsWith(' ') ? ONE : ZERO;
      const newEmotePosition = cursorPos + prefixLength;

      // Create new chips array with updated positions
      const newChips: EmoteChip[] = [];
      const lengthDiff = next.text.length - text.length;

      for (const chip of emoteChips) {
        // Use cursorPos (insertion point), not newEmotePosition, for comparison
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
    },
    [
      disabled,
      text,
      getEmoteImageUrl,
      setComposerText,
      setCursorPosition,
      closeSuggestions,
      emoteChips,
      getCursorPosition,
    ],
  );

  const clearPreview = useCallback((): void => {
    setPreviewOpen(false);
    if (previewTimerRef.current !== null) {
      clearTimeout(previewTimerRef.current);
      previewTimerRef.current = null;
    }
  }, []);

  return {
    text,
    emoteChips,
    suggestionsOpen,
    suggestionItems,
    suggestionIndex,
    previewOpen,
    previewUrl,
    previewPosition,
    composerRef,
    previewTimerRef,
    handleInput,
    handlePaste,
    handleKeydown,
    handleSuggestionClick,
    insertEmote,
    submit,
    closeSuggestions,
    clearPreview,
    createEmoteImageElement,
    renderComposerContent,
    readComposerModel,
    setComposerText,
  };
}
