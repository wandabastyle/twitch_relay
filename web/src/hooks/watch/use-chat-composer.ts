import { useCallback, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import type { EmoteItem } from '../../api-client';
import { useComposerContent } from './chat-composer-content';
import { setCursorPositionBase } from './chat-composer-cursor';
import { getCursorPosition as getCursorPositionUtil } from './chat-composer-cursor-position';
import {
  createEmoteImageElement as createEmoteImageElementBase,
  type CreateEmoteImageElementOptions,
} from './chat-composer-emotes';
import { useInsertEmote } from './chat-composer-insert';
import { createKeyboardHandlers, createPasteHandler } from './chat-composer-keyboard';
import {
  startPreviewTimer,
  endPreview,
  clearPreview as clearPreviewBase,
  type PreviewPosition,
} from './chat-composer-preview';
import { refreshSuggestions as refreshSuggestionsBase } from './chat-composer-suggestions';

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

interface ComposerUiState {
  previewOpen: boolean;
  previewPosition: PreviewPosition;
  previewUrl: string;
  setPreviewOpen: Dispatch<SetStateAction<boolean>>;
  setPreviewPosition: Dispatch<SetStateAction<PreviewPosition>>;
  setPreviewUrl: Dispatch<SetStateAction<string>>;
  setSuggestionIndex: Dispatch<SetStateAction<number>>;
  setSuggestionItems: Dispatch<SetStateAction<EmoteItem[]>>;
  setSuggestionsOpen: Dispatch<SetStateAction<boolean>>;
  suggestionIndex: number;
  suggestionItems: EmoteItem[];
  suggestionsOpen: boolean;
}

const useComposerUiState = (): ComposerUiState => {
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewUrl, setPreviewUrl] = useState('');
  const [previewPosition, setPreviewPosition] = useState<PreviewPosition>({
    left: ZERO,
    top: ZERO,
  });
  const [suggestionsOpen, setSuggestionsOpen] = useState(false);
  const [suggestionItems, setSuggestionItems] = useState<EmoteItem[]>([]);
  const [suggestionIndex, setSuggestionIndex] = useState(ZERO);

  return {
    previewOpen,
    previewPosition,
    previewUrl,
    setPreviewOpen,
    setPreviewPosition,
    setPreviewUrl,
    setSuggestionIndex,
    setSuggestionItems,
    setSuggestionsOpen,
    suggestionIndex,
    suggestionItems,
    suggestionsOpen,
  };
};

const useEmoteImageElementFactory = (
  previewTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>,
  ui: Pick<ComposerUiState, 'setPreviewOpen' | 'setPreviewPosition' | 'setPreviewUrl'>,
): ((code: string, imageUrl: string) => HTMLSpanElement) =>
  useCallback(
    (code: string, imageUrl: string): HTMLSpanElement => {
      const opts: CreateEmoteImageElementOptions = {
        code,
        imageUrl,
        onPreviewEnd: () => {
          endPreview({ previewTimerRef, setPreviewOpen: ui.setPreviewOpen });
        },
        onPreviewStart: (rect, imgUrl) => {
          startPreviewTimer({
            imageUrl: imgUrl,
            previewDelayMs: PREVIEW_DELAY_MS,
            previewTimerRef,
            rect,
            setPreviewOpen: ui.setPreviewOpen,
            setPreviewPosition: ui.setPreviewPosition,
            setPreviewUrl: ui.setPreviewUrl,
          });
        },
        previewDelayMs: PREVIEW_DELAY_MS,
      };
      return createEmoteImageElementBase(opts);
    },
    [previewTimerRef, ui],
  );

export const useChatComposer = (options: UseChatComposerOptions): UseChatComposerReturn => {
  const { availableEmotes, disabled = false, onSubmit } = options;

  const composerRef = useRef<HTMLDivElement>(null);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const ui = useComposerUiState();
  const createEmoteImageElement = useEmoteImageElementFactory(previewTimerRef, ui);

  const content = useComposerContent(composerRef, createEmoteImageElement);
  const { text, emoteChips, setComposerText, readComposerModel, renderComposerContent } = content;

  const getCursorPosition = useCallback(
    (): number => getCursorPositionUtil({ composerRef, text }),
    [composerRef, text],
  );

  const setCursorPosition = useCallback(
    (position: number): void => {
      setCursorPositionBase({ composerElement: composerRef.current, position });
    },
    [composerRef],
  );

  const closeSuggestions = useCallback((): void => {
    ui.setSuggestionsOpen(false);
    ui.setSuggestionItems([]);
    ui.setSuggestionIndex(ZERO);
  }, [ui]);

  const refreshSuggestions = useCallback((): void => {
    refreshSuggestionsBase({
      availableEmotes,
      closeSuggestions,
      getCursorPosition,
      setSuggestionIndex: ui.setSuggestionIndex,
      setSuggestionItems: ui.setSuggestionItems,
      setSuggestionsOpen: ui.setSuggestionsOpen,
      text,
    });
  }, [text, availableEmotes, getCursorPosition, closeSuggestions, ui]);

  const submit = useCallback((): void => {
    const trimmed = text.trim();
    if (trimmed === '' || disabled) {
      return;
    }
    onSubmit(trimmed);
    setComposerText('', []);
    closeSuggestions();
  }, [text, disabled, onSubmit, setComposerText, closeSuggestions]);

  const keyboardHandlers = createKeyboardHandlers(
    {
      availableEmotes,
      composerRef,
      disabled,
      emoteChips,
      suggestionIndex: ui.suggestionIndex,
      suggestionItems: ui.suggestionItems,
      suggestionsOpen: ui.suggestionsOpen,
      text,
    },
    {
      closeSuggestions,
      onSubmit,
      setComposerText,
      setSuggestionIndex: ui.setSuggestionIndex,
      setSuggestionItems: ui.setSuggestionItems,
      setSuggestionsOpen: ui.setSuggestionsOpen,
    },
  );

  const pasteHandler = createPasteHandler({
    composerRef,
    refreshSuggestions,
    setComposerText,
    text,
  });

  const insertEmoteHook = useInsertEmote(
    { availableEmotes, composerRef, disabled, emoteChips, text },
    { closeSuggestions, setComposerText },
    { getCursorPosition, setCursorPosition },
  );

  const handleInput = useCallback((): void => {
    const cursorPos = getCursorPosition();
    const model = readComposerModel();
    content.setText(model.text);
    content.setEmoteChips(model.chips);
    refreshSuggestions();
    if (cursorPos !== text.length) {
      setCursorPosition(cursorPos);
    }
  }, [
    getCursorPosition,
    readComposerModel,
    refreshSuggestions,
    text.length,
    setCursorPosition,
    content,
  ]);

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
    clearPreviewBase({ previewTimerRef, setPreviewOpen: ui.setPreviewOpen });
  }, [ui.setPreviewOpen]);

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
    previewOpen: ui.previewOpen,
    previewPosition: ui.previewPosition,
    previewTimerRef,
    previewUrl: ui.previewUrl,
    readComposerModel,
    renderComposerContent,
    setComposerText,
    submit,
    suggestionIndex: ui.suggestionIndex,
    suggestionItems: ui.suggestionItems,
    suggestionsOpen: ui.suggestionsOpen,
    text,
  };
};
