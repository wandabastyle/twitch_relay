import type { EmoteItem } from '../../api-client';
import {
  normalizeSingleLine,
  trimToMaxLength,
} from '../../lib/components/watch/chat-composer-helpers.svelte';
import {
  getRangeTextLength as getRangeTextLengthBase,
  setCursorPositionBase,
} from './chat-composer-cursor';
import {
  applySuggestion as applySuggestionBase,
  hasActiveSelection,
  selectCurrentSuggestion as selectCurrentSuggestionBase,
  moveSelection as moveSelectionBase,
  handleEnterKey as handleEnterKeyBase,
  handleInactiveSelectionKey,
  handleSelectionKey as handleSelectionKeyBase,
  handleEscapeKey as handleEscapeKeyBase,
  findCurrentQuery,
  refreshSuggestions as refreshSuggestionsBase,
} from './chat-composer-suggestions';
import type { EmoteChip } from './use-chat-composer';

const MAX_TEXT_LENGTH = 500;
const ZERO = 0;

export interface GetSelectionRangeResult {
  range: Range | null;
  selection: Selection | null;
}

export const getSelectionRange = (
  composerRef: React.RefObject<HTMLDivElement | null>,
): GetSelectionRangeResult => {
  const selection = globalThis.getSelection();
  if (selection === null || composerRef.current === null || selection.rangeCount === ZERO) {
    return { range: null, selection: null };
  }
  const range = selection.getRangeAt(ZERO);
  return composerRef.current.contains(range.commonAncestorContainer)
    ? { range, selection }
    : { range: null, selection: null };
};

export interface CursorUtilsDeps {
  composerRef: React.RefObject<HTMLDivElement | null>;
  text: string;
  textLength: number;
}

export const getCursorPosition = (deps: CursorUtilsDeps): number => {
  const { composerRef, text, textLength } = deps;
  const { range } = getSelectionRange(composerRef);
  return range === null
    ? textLength
    : getRangeTextLengthBase({
        composerElement: composerRef.current,
        fallbackLength: text.length,
        range,
      });
};

export const setCursorPosition = (
  composerRef: React.RefObject<HTMLDivElement | null>,
  position: number,
): void => {
  setCursorPositionBase({
    composerElement: composerRef.current,
    position,
  });
};

export interface SuggestionLogicDeps {
  text: string;
  emoteChips: EmoteChip[];
  availableEmotes: EmoteItem[];
  composerRef: React.RefObject<HTMLDivElement | null>;
}

export interface SuggestionLogicState {
  suggestionsOpen: boolean;
  suggestionItems: EmoteItem[];
  suggestionIndex: number;
}

export interface SuggestionLogicActions {
  setSuggestionsOpen: (open: boolean) => void;
  setSuggestionItems: (items: EmoteItem[]) => void;
  setSuggestionIndex: (index: number | ((prev: number) => number)) => void;
  closeSuggestions: () => void;
  setComposerText: (text: string, chips?: EmoteChip[]) => void;
}

export interface CreateSuggestionLogicReturn {
  applySuggestion: (code: string) => void;
  refreshSuggestions: () => void;
  selectSuggestion: (item: EmoteItem) => void;
}

export const createSuggestionLogic = (
  deps: SuggestionLogicDeps,
  actions: SuggestionLogicActions,
): CreateSuggestionLogicReturn => {
  const { text, emoteChips, availableEmotes, composerRef } = deps;

  const getEmoteImageUrl = (code: string): string | null => {
    const emote = availableEmotes.find((item) => item.code === code);
    return emote?.image_url ?? null;
  };

  const applySuggestionLogic = (code: string): void => {
    const cursorPos = getCursorPosition({ composerRef, text, textLength: text.length });
    const query = findCurrentQuery({ getCursorPosition: () => cursorPos, text });
    if (query === null) {
      actions.closeSuggestions();
      return;
    }

    applySuggestionBase({
      closeSuggestions: actions.closeSuggestions,
      code,
      emoteChips,
      getEmoteImageUrl,
      query,
      setComposerText: actions.setComposerText,
      setCursorPosition: (pos) => {
        setCursorPosition(composerRef, pos);
      },
      text,
    });
  };

  const selectSuggestion = (item: EmoteItem): void => {
    const safeCode = item.code.trim();
    if (safeCode === '') {
      actions.closeSuggestions();
      return;
    }
    applySuggestionLogic(safeCode);
  };

  const refreshSuggestions = (): void => {
    const cursorPos = getCursorPosition({ composerRef, text, textLength: text.length });
    refreshSuggestionsBase({
      availableEmotes,
      closeSuggestions: actions.closeSuggestions,
      getCursorPosition: () => cursorPos,
      setSuggestionIndex: actions.setSuggestionIndex,
      setSuggestionItems: actions.setSuggestionItems,
      setSuggestionsOpen: actions.setSuggestionsOpen,
      text,
    });
  };

  return {
    applySuggestion: applySuggestionLogic,
    refreshSuggestions,
    selectSuggestion,
  };
};

export interface KeyboardHandlerDeps {
  composerRef: React.RefObject<HTMLDivElement | null>;
  text: string;
  emoteChips: EmoteChip[];
  availableEmotes: EmoteItem[];
  suggestionsOpen: boolean;
  suggestionItems: EmoteItem[];
  suggestionIndex: number;
  disabled: boolean;
}

export interface KeyboardHandlerActions {
  closeSuggestions: () => void;
  setComposerText: (text: string, chips?: EmoteChip[]) => void;
  setSuggestionIndex: (index: number | ((prev: number) => number)) => void;
  setSuggestionsOpen: (open: boolean) => void;
  setSuggestionItems: (items: EmoteItem[]) => void;
  onSubmit: (text: string) => void;
}

export interface CreateKeyboardHandlersReturn {
  handleKeydown: (event: React.KeyboardEvent) => void;
  refreshSuggestions: () => void;
  selectCurrent: () => void;
  selectSuggestion: (item: EmoteItem) => void;
}

export const createKeyboardHandlers = (
  deps: KeyboardHandlerDeps,
  actions: KeyboardHandlerActions,
): CreateKeyboardHandlersReturn => {
  const {
    composerRef,
    text,
    emoteChips,
    availableEmotes,
    suggestionsOpen,
    suggestionItems,
    suggestionIndex,
    disabled,
  } = deps;

  const { selectSuggestion, refreshSuggestions } = createSuggestionLogic(
    { availableEmotes, composerRef, emoteChips, text },
    actions,
  );

  const submit = (): void => {
    const trimmed = text.trim();
    if (trimmed === '' || disabled) {
      return;
    }
    actions.onSubmit(trimmed);
    actions.setComposerText('', []);
    actions.closeSuggestions();
  };

  const selectCurrent = (): void => {
    selectCurrentSuggestionBase(suggestionItems, suggestionIndex, selectSuggestion);
  };

  const moveSelectionBy = (delta: number): void => {
    moveSelectionBase(delta, suggestionItems.length, actions.setSuggestionIndex);
  };

  const handleEnterKey = (event: React.KeyboardEvent, activeSelection: boolean): boolean =>
    handleEnterKeyBase(event, activeSelection, selectCurrent, submit);

  const handleSelectionKey = (event: React.KeyboardEvent): boolean =>
    handleSelectionKeyBase(event, selectCurrent, moveSelectionBy);

  const handleEscapeKey = (event: React.KeyboardEvent): void => {
    handleEscapeKeyBase(event, actions.closeSuggestions);
  };

  const handleKeydown = (event: React.KeyboardEvent): void => {
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
  };

  return {
    handleKeydown,
    refreshSuggestions,
    selectCurrent,
    selectSuggestion,
  };
};

export interface PasteHandlerDeps {
  composerRef: React.RefObject<HTMLDivElement | null>;
  text: string;
  setComposerText: (text: string, chips?: EmoteChip[]) => void;
  refreshSuggestions: () => void;
}

export const createPasteHandler = (deps: PasteHandlerDeps) => {
  const { composerRef, text, setComposerText, refreshSuggestions } = deps;

  return (event: React.ClipboardEvent): void => {
    event.preventDefault();
    const pasted = event.clipboardData.getData('text/plain');
    const cleaned = normalizeSingleLine(pasted);
    if (cleaned === '') {
      return;
    }

    const cursorPos = getCursorPosition({ composerRef, text, textLength: text.length });
    const nextText = trimToMaxLength(
      text.slice(ZERO, cursorPos) + cleaned + text.slice(cursorPos),
      MAX_TEXT_LENGTH,
    );
    setComposerText(nextText);
    setCursorPosition(composerRef, Math.min(cursorPos + cleaned.length, nextText.length));
    refreshSuggestions();
  };
};
