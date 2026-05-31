import type { EmoteItem } from '../../api-client';
import {
  findActiveEmoteQuery,
  rankSuggestions,
  replaceRangeWithEmote,
  type ActiveEmoteQuery,
} from '../../lib/components/watch/chat-composer-helpers.svelte';
import type { EmoteChip } from './use-chat-composer';

const ZERO = 0;
const ONE = 1;
const MAX_SUGGESTIONS = 10;

export interface FindCurrentQueryOptions {
  text: string;
  getCursorPosition: () => number;
}

export function findCurrentQuery(options: FindCurrentQueryOptions): ActiveEmoteQuery | null {
  const { text, getCursorPosition } = options;
  return findActiveEmoteQuery(text, getCursorPosition());
}

export interface ApplySuggestionOptions {
  text: string;
  emoteChips: EmoteChip[];
  query: ActiveEmoteQuery;
  code: string;
  getEmoteImageUrl: (code: string) => string | null;
  setComposerText: (text: string, chips: EmoteChip[]) => void;
  setCursorPosition: (position: number) => void;
  closeSuggestions: () => void;
}

export function applySuggestion(options: ApplySuggestionOptions): void {
  const {
    text,
    emoteChips,
    query,
    code,
    getEmoteImageUrl,
    setComposerText,
    setCursorPosition,
    closeSuggestions,
  } = options;

  const next = replaceRangeWithEmote(text, query, code);
  const imageUrl = getEmoteImageUrl(code);

  if (imageUrl === null) {
    // Emote not found, just set text without chips
    setComposerText(next.text, []);
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
}

export interface RefreshSuggestionsOptions {
  text: string;
  availableEmotes: EmoteItem[];
  getCursorPosition: () => number;
  setSuggestionsOpen: (open: boolean) => void;
  setSuggestionItems: (items: EmoteItem[]) => void;
  setSuggestionIndex: (index: number | ((prev: number) => number)) => void;
  closeSuggestions: () => void;
}

export function refreshSuggestions(options: RefreshSuggestionsOptions): void {
  const {
    text,
    availableEmotes,
    getCursorPosition,
    setSuggestionsOpen,
    setSuggestionItems,
    setSuggestionIndex,
    closeSuggestions,
  } = options;

  const query = findActiveEmoteQuery(text, getCursorPosition());
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
}

export interface SuggestionKeyboardHandlersOptions {
  suggestionsOpen: boolean;
  suggestionItems: EmoteItem[];
  suggestionIndex: number;
  selectCurrentSuggestion: () => void;
  moveSelection: (delta: number) => void;
  closeSuggestions: () => void;
}

export interface HandleKeydownResult {
  handled: boolean;
}

export function hasActiveSelection(suggestionsOpen: boolean, suggestionItemsLength: number): boolean {
  return suggestionsOpen && suggestionItemsLength > ZERO;
}

export function selectCurrentSuggestion(
  suggestionItems: EmoteItem[],
  suggestionIndex: number,
  selectSuggestion: (item: EmoteItem) => void,
): void {
  const selected = suggestionItems[suggestionIndex];
  if (selected !== undefined) {
    selectSuggestion(selected);
  }
}

export function moveSelection(
  delta: number,
  suggestionItemsLength: number,
  setSuggestionIndex: (index: number | ((prev: number) => number)) => void,
): void {
  setSuggestionIndex((prev) => (prev + delta + suggestionItemsLength) % suggestionItemsLength);
}

export function handleEnterKey(
  event: React.KeyboardEvent,
  activeSelection: boolean,
  selectCurrentSuggestion: () => void,
  submit: () => void,
): boolean {
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
}

export function handleInactiveSelectionKey(event: React.KeyboardEvent): boolean {
  if (event.key !== 'Escape') {
    return false;
  }
  event.preventDefault();
  return true;
}

export function handleSelectionKey(
  event: React.KeyboardEvent,
  selectCurrentSuggestion: () => void,
  moveSelection: (delta: number) => void,
): boolean {
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
}

export function handleEscapeKey(
  event: React.KeyboardEvent,
  closeSuggestions: () => void,
): void {
  if (event.key === 'Escape') {
    event.preventDefault();
    closeSuggestions();
  }
}
