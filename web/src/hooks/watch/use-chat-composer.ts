import { useCallback, useRef, useState } from 'react';
import type { EmoteItem } from '../../api-client';
import {
  createEmoteImageElement as createEmoteImageElementBase,
  type CreateEmoteImageElementOptions,
} from './chat-composer-emotes';
import { refreshSuggestions as refreshSuggestionsBase } from './chat-composer-suggestions';
import {
  startPreviewTimer,
  endPreview,
  clearPreview as clearPreviewBase,
  type PreviewPosition,
} from './chat-composer-preview';
import { setCursorPositionBase } from './chat-composer-cursor';
import { createKeyboardHandlers, createPasteHandler, getSelectionRange } from './chat-composer-keyboard';
import { useInsertEmote } from './chat-composer-insert';
import { useComposerContent } from './chat-composer-content';

const ZERO = 0;
const PREVIEW_DELAY_MS = 350;

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

  // Preview state
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewUrl, setPreviewUrl] = useState('');
  const [previewPosition, setPreviewPosition] = useState<PreviewPosition>({ left: ZERO, top: ZERO });

  // Suggestions state
  const [suggestionsOpen, setSuggestionsOpen] = useState(false);
  const [suggestionItems, setSuggestionItems] = useState<EmoteItem[]>([]);
  const [suggestionIndex, setSuggestionIndex] = useState(ZERO);

  // Create emote element with preview handlers
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
    [],
  );

  // Content hook
  const content = useComposerContent(composerRef, createEmoteImageElement);
  const { text, emoteChips, setComposerText, readComposerModel, renderComposerContent } = content;

  // Cursor utilities
  const getCursorPosition = useCallback((): number => {
    const { range } = getSelectionRange(composerRef);
    if (range === null) { return text.length; }
    // Calculate text length within range
    const composerElement = composerRef.current;
    if (composerElement === null) { return text.length; }
    const preRange = document.createRange();
    preRange.setStart(composerElement, ZERO);
    preRange.setEnd(range.startContainer, range.startOffset);
    const div = document.createElement('div');
    div.append(preRange.cloneContents());
    // Walk nodes and count text length
    let length = ZERO;
    const nodeFilter = NodeFilter.SHOW_ELEMENT + NodeFilter.SHOW_TEXT;
    const walker = document.createTreeWalker(div, nodeFilter, null);
    while (walker.nextNode()) {
      const node = walker.currentNode;
      if (node.nodeType === Node.TEXT_NODE) {
        length += node.textContent?.length ?? ZERO;
      } else if (node.nodeType === Node.ELEMENT_NODE && node instanceof HTMLImageElement) {
        const { code } = node.dataset;
        if (code !== undefined && code !== '') { length += code.length; }
      }
    }
    return length;
  }, [text.length]);

  const setCursorPosition = useCallback(
    (position: number): void => {
      setCursorPositionBase({ composerElement: composerRef.current, position });
    },
    [],
  );

  // Suggestions handlers
  const closeSuggestions = useCallback((): void => {
    setSuggestionsOpen(false);
    setSuggestionItems([]);
    setSuggestionIndex(ZERO);
  }, []);

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

  // Submit handler
  const submit = useCallback((): void => {
    const trimmed = text.trim();
    if (trimmed === '' || disabled) { return; }
    onSubmit(trimmed);
    setComposerText('', []);
    closeSuggestions();
  }, [text, disabled, onSubmit, setComposerText, closeSuggestions]);

  // Keyboard handlers
  const keyboardHandlers = createKeyboardHandlers(
    {
      availableEmotes,
      composerRef,
      disabled,
      emoteChips,
      suggestionIndex,
      suggestionItems,
      suggestionsOpen,
      text,
    },
    {
      closeSuggestions,
      onSubmit,
      setComposerText,
      setSuggestionIndex,
      setSuggestionItems,
      setSuggestionsOpen,
    },
  );

  const pasteHandler = createPasteHandler({
    composerRef,
    refreshSuggestions,
    setComposerText,
    text,
  });

  // Insert emote hook
  const insertEmoteHook = useInsertEmote(
    { availableEmotes, composerRef, disabled, emoteChips, text },
    { closeSuggestions, setComposerText },
    { getCursorPosition, setCursorPosition },
  );

  // Event handlers
  const handleInput = useCallback((): void => {
    const cursorPos = getCursorPosition();
    const model = readComposerModel();
    content.setText(model.text);
    content.setEmoteChips(model.chips);
    refreshSuggestions();
    if (cursorPos !== text.length) { setCursorPosition(cursorPos); }
  }, [getCursorPosition, readComposerModel, refreshSuggestions, text.length, setCursorPosition, content]);

  const handleKeydown = useCallback(
    (event: React.KeyboardEvent): void => {
      keyboardHandlers.handleKeydown(event);
    },
    [keyboardHandlers],
  );

  const handlePaste = useCallback(
    (event: React.ClipboardEvent): void => {
      pasteHandler(event);
    },
    [pasteHandler],
  );

  const handleSuggestionClick = useCallback(
    (item: EmoteItem): void => {
      const safeCode = item.code.trim();
      if (safeCode === '') {
        closeSuggestions();
        return;
      }
      void item;
    },
    [closeSuggestions],
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
    insertEmote: insertEmoteHook.insertEmote,
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
