import { useCallback, useRef, useState } from 'react';
import type { EmoteItem } from '../../api-client';
import {
  insertCodeAtCursor,
  normalizeSingleLine,
  trimToMaxLength,
} from '../../lib/components/watch/chat-composer-helpers.svelte';
import {
  createEmoteImageElement as createEmoteImageElementBase,
  renderComposerContent as renderComposerContentBase,
  readComposerModel as readComposerModelBase,
  type CreateEmoteImageElementOptions,
} from './chat-composer-emotes';
import {
  applySuggestion as applySuggestionBase,
  refreshSuggestions as refreshSuggestionsBase,
  hasActiveSelection,
  selectCurrentSuggestion as selectCurrentSuggestionBase,
  moveSelection as moveSelectionBase,
  handleEnterKey as handleEnterKeyBase,
  handleInactiveSelectionKey,
  handleSelectionKey as handleSelectionKeyBase,
  handleEscapeKey as handleEscapeKeyBase,
  findCurrentQuery,
} from './chat-composer-suggestions';
import {
  startPreviewTimer,
  endPreview,
  clearPreview as clearPreviewBase,
  type PreviewPosition,
} from './chat-composer-preview';
import {
  getRangeTextLength as getRangeTextLengthBase,
  setCursorPositionBase,
  insertEmoteChip,
} from './chat-composer-cursor';

const MAX_TEXT_LENGTH = 500;
const ONE = 1;
const ZERO = 0;

export interface EmoteChip {
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

export const useChatComposer = (options: UseChatComposerOptions): UseChatComposerReturn => {
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
  const [previewPosition, setPreviewPosition] = useState<PreviewPosition>({ left: ZERO, top: ZERO });

  const PREVIEW_DELAY_MS = 350;

  const getEmoteImageUrl = useCallback(
    (code: string): string | null => {
      const emote = availableEmotes.find((emoteItem) => emoteItem.code === code);
      return emote?.image_url ?? null;
    },
    [availableEmotes],
  );

  const createEmoteImageElement = useCallback(
    (code: string, imageUrl: string): HTMLSpanElement => {
      const emoteElementOptions: CreateEmoteImageElementOptions = {
        code,
        imageUrl,
        onPreviewEnd: () => {
          endPreview({ previewTimerRef, setPreviewOpen });
        },
        onPreviewStart: (rect, imgUrl) => {
          startPreviewTimer({
            imageUrl: imgUrl,
            previewDelayMs: PREVIEW_DELAY_MS,
            previewTimerRef,
            rect,
            setPreviewOpen,
            setPreviewPosition,
            setPreviewUrl,
          });
        },
        previewDelayMs: PREVIEW_DELAY_MS,
      };
      return createEmoteImageElementBase(emoteElementOptions);
    },
    [PREVIEW_DELAY_MS],
  );

  const renderComposerContent = useCallback(
    (textValue: string, chips: EmoteChip[]): void => {
      renderComposerContentBase({
        chips,
        composerElement: composerRef.current,
        createEmoteElement: createEmoteImageElement,
        textValue,
      });
    },
    [createEmoteImageElement],
  );

  const readComposerModel = useCallback((): { chips: EmoteChip[]; text: string } =>
    readComposerModelBase(composerRef.current),
  []);

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
      (range: Range): number =>
        getRangeTextLengthBase({
          composerElement: composerRef.current,
          fallbackLength: text.length,
          range,
        }),
      [text.length],
    );

  const getCursorPosition = useCallback((): number => {
    const range = getSelectionRange();
    return range === null ? text.length : getRangeTextLength(range);
  }, [getSelectionRange, getRangeTextLength, text.length]);

  const setCursorPosition = useCallback(
    (position: number): void => {
      setCursorPositionBase({
        composerElement: composerRef.current,
        position,
      });
    },
    [],
  );

  const closeSuggestions = useCallback((): void => {
    setSuggestionsOpen(false);
    setSuggestionItems([]);
    setSuggestionIndex(ZERO);
  }, []);

  const applySuggestion = useCallback(
    (code: string): void => {
      const query = findCurrentQuery({ getCursorPosition, text });
      if (query === null) {
        closeSuggestions();
        return;
      }

      applySuggestionBase({
        closeSuggestions,
        code,
        emoteChips,
        getEmoteImageUrl,
        query,
        setComposerText,
        setCursorPosition,
        text,
      });
    },
    [text, emoteChips, getEmoteImageUrl, setComposerText, setCursorPosition, closeSuggestions, getCursorPosition],
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
    refreshSuggestionsBase({
      availableEmotes,
      closeSuggestions,
      getCursorPosition,
      setSuggestionIndex,
      setSuggestionItems,
      setSuggestionsOpen,
      text,
    });
  }, [text, availableEmotes, getCursorPosition, closeSuggestions]);

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

  const selectSuggestionCurrent = useCallback((): void => {
    selectCurrentSuggestionBase(suggestionItems, suggestionIndex, selectSuggestion);
  }, [suggestionItems, suggestionIndex, selectSuggestion]);

  const moveSelectionBy = useCallback(
    (delta: number): void => {
      moveSelectionBase(delta, suggestionItems.length, setSuggestionIndex);
    },
    [suggestionItems.length],
  );

  const handleEnterKey = useCallback(
    (event: React.KeyboardEvent, activeSelection: boolean): boolean =>
      handleEnterKeyBase(event, activeSelection, selectSuggestionCurrent, submit),
    [selectSuggestionCurrent, submit],
  );

  const handleSelectionKey = useCallback(
    (event: React.KeyboardEvent): boolean =>
      handleSelectionKeyBase(event, selectSuggestionCurrent, moveSelectionBy),
    [selectSuggestionCurrent, moveSelectionBy],
  );

  const handleEscapeKey = useCallback(
    (event: React.KeyboardEvent): void => {
      handleEscapeKeyBase(event, closeSuggestions);
    },
    [closeSuggestions],
  );

  const handleKeydown = useCallback(
    (event: React.KeyboardEvent): void => {
      const activeSelection = hasActiveSelection(suggestionsOpen, suggestionItems.length);
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
      suggestionsOpen,
      suggestionItems.length,
      handleEnterKey,
      handleSelectionKey,
      handleEscapeKey,
    ],
  );

  const handlePaste = useCallback(
    (event: React.ClipboardEvent): void => {
      event.preventDefault();
      // ClipboardData is always present in ClipboardEvent - guarded by event type
      const pasted = event.clipboardData.getData('text/plain');
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
        const newChips = insertEmoteChip({
          cursorPos,
          emoteChips,
          imageUrl,
          lengthDiff: next.text.length - text.length,
          newEmotePosition,
          safeCode,
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
    clearPreviewBase({ previewTimerRef, setPreviewOpen });
  }, []);

  return {
    clearPreview,
    closeSuggestions,
    composerRef,
    createEmoteImageElement,
    emoteChips,
    handleInput,
    handleKeydown,
    handlePaste,
    handleSuggestionClick,
    insertEmote,
    previewOpen,
    previewPosition,
    previewTimerRef,
    previewUrl,
    readComposerModel,
    renderComposerContent,
    setComposerText,
    submit,
    suggestionIndex,
    suggestionItems,
    suggestionsOpen,
    text,
  };
};
