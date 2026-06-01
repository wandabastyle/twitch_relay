import { useCallback } from 'react';
import type { EmoteItem } from '../../api-client';
import { insertEmoteChip, setCursorPositionBase } from './chat-composer-cursor';
import { insertCodeAtCursor } from './chat-composer-helpers';
import type { EmoteChip } from './use-chat-composer';

const MAX_TEXT_LENGTH = 500;
const ZERO = 0;
const ONE = 1;

export interface InsertEmoteDeps {
  composerRef: React.RefObject<HTMLDivElement | null>;
  text: string;
  emoteChips: EmoteChip[];
  disabled: boolean;
  availableEmotes: EmoteItem[];
}

export interface InsertEmoteActions {
  setComposerText: (text: string, chips?: EmoteChip[]) => void;
  closeSuggestions: () => void;
}

export interface CursorUtils {
  getCursorPosition: () => number;
  setCursorPosition: (position: number) => void;
}

export interface UseInsertEmoteReturn {
  insertEmote: (code: string) => void;
}

export const useInsertEmote = (
  deps: InsertEmoteDeps,
  actions: InsertEmoteActions,
  cursorUtils: CursorUtils,
): UseInsertEmoteReturn => {
  const { composerRef, text, emoteChips, disabled, availableEmotes } = deps;
  const { getCursorPosition, setCursorPosition } = cursorUtils;

  const getEmoteImageUrl = useCallback(
    (code: string): string | null => {
      const emote = availableEmotes.find((item) => item.code === code);
      return emote?.image_url ?? null;
    },
    [availableEmotes],
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
        actions.setComposerText(next.text);
        setCursorPosition(next.cursor);
        actions.closeSuggestions();
        return;
      }

      const before = text.slice(ZERO, cursorPos);
      const prefixLength = before.length > ZERO && !before.endsWith(' ') ? ONE : ZERO;
      const newEmotePosition = cursorPos + prefixLength;

      const newChips = insertEmoteChip({
        cursorPos,
        emoteChips,
        imageUrl,
        lengthDiff: next.text.length - text.length,
        newEmotePosition,
        safeCode,
      });

      actions.setComposerText(next.text, newChips);
      setCursorPosition(next.cursor);
      actions.closeSuggestions();
    },
    [
      composerRef,
      disabled,
      text,
      emoteChips,
      getEmoteImageUrl,
      getCursorPosition,
      setCursorPosition,
      actions,
    ],
  );

  return { insertEmote };
};

export interface CreateInsertEmoteOptions {
  composerRef: React.RefObject<HTMLDivElement | null>;
  getCursorPosition: () => number;
  getEmoteImageUrl: (code: string) => string | null;
  disabled: boolean;
}

export const createInsertEmote = (
  options: CreateInsertEmoteOptions,
  setComposerText: (text: string, chips?: EmoteChip[]) => void,
  closeSuggestions: () => void,
) => {
  const { composerRef, getCursorPosition, getEmoteImageUrl, disabled } = options;

  return (code: string, text: string, emoteChips: EmoteChip[]): void => {
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
      setComposerText(next.text);
      setCursorPositionBase({ composerElement: composerRef.current, position: next.cursor });
      closeSuggestions();
      return;
    }

    const before = text.slice(ZERO, cursorPos);
    const prefixLength = before.length > ZERO && !before.endsWith(' ') ? ONE : ZERO;
    const newEmotePosition = cursorPos + prefixLength;

    const newChips = insertEmoteChip({
      cursorPos,
      emoteChips,
      imageUrl,
      lengthDiff: next.text.length - text.length,
      newEmotePosition,
      safeCode,
    });

    setComposerText(next.text, newChips);
    setCursorPositionBase({ composerElement: composerRef.current, position: next.cursor });
    closeSuggestions();
  };
};
