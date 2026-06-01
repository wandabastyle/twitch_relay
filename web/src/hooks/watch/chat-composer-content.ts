import { useCallback, useState } from 'react';
import {
  renderComposerContent as renderComposerContentBase,
  readComposerModel as readComposerModelBase,
} from './chat-composer-emotes';
import type { EmoteChip } from './use-chat-composer';

export interface UseComposerContentReturn {
  text: string;
  emoteChips: EmoteChip[];
  setText: (text: string) => void;
  setEmoteChips: (chips: EmoteChip[]) => void;
  setComposerText: (value: string, chips?: EmoteChip[]) => void;
  renderComposerContent: (textValue: string, chips: EmoteChip[]) => void;
  readComposerModel: () => { text: string; chips: EmoteChip[] };
}

export const useComposerContent = (
  composerRef: React.RefObject<HTMLDivElement | null>,
  createEmoteElement: (code: string, imageUrl: string) => HTMLSpanElement,
): UseComposerContentReturn => {
  const [text, setTextState] = useState('');
  const [emoteChips, setEmoteChipsState] = useState<EmoteChip[]>([]);

  const setText = useCallback((newText: string): void => {
    setTextState(newText);
  }, []);

  const setEmoteChips = useCallback((newChips: EmoteChip[]): void => {
    setEmoteChipsState(newChips);
  }, []);

  const setComposerText = useCallback(
    (value: string, chips: EmoteChip[] = []): void => {
      setTextState(value);
      setEmoteChipsState(chips);
      renderComposerContentBase({
        chips,
        composerElement: composerRef.current,
        createEmoteElement,
        textValue: value,
      });
    },
    [composerRef, createEmoteElement],
  );

  const renderComposerContent = useCallback(
    (textValue: string, chips: EmoteChip[]): void => {
      renderComposerContentBase({
        chips,
        composerElement: composerRef.current,
        createEmoteElement,
        textValue,
      });
    },
    [composerRef, createEmoteElement],
  );

  const readComposerModel = useCallback(
    (): { chips: EmoteChip[]; text: string } => readComposerModelBase(composerRef.current),
    [composerRef],
  );

  return {
    emoteChips,
    readComposerModel,
    renderComposerContent,
    setComposerText,
    setEmoteChips,
    setText,
    text,
  };
};
