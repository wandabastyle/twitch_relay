const ZERO = 0;
const ONE = 1;
const TWO = 2;

export interface ActiveEmoteQuery {
  readonly end: number;
  readonly query: string;
  readonly start: number;
}

export interface EmoteItem {
  readonly code: string;
  readonly id: string;
  readonly image_url: string;
}

const EMOTE_PATTERN = /(^|\s):([A-Za-z0-9_]{2,})$/;
const NO_MATCH_SCORE = 99;

interface RankedEntry {
  readonly item: EmoteItem;
  readonly score: number;
}

const scoreEmote = (code: string, query: string): number => {
  const loweredCode = code.toLowerCase();
  const loweredQuery = query.toLowerCase();
  if (loweredCode === loweredQuery) {
    return ZERO;
  }
  if (loweredCode.startsWith(loweredQuery)) {
    return ONE;
  }
  if (loweredCode.includes(loweredQuery)) {
    return TWO;
  }
  return NO_MATCH_SCORE;
};

const compareRankedEntries = (
  left: Readonly<RankedEntry>,
  right: Readonly<RankedEntry>,
): number => {
  if (left.score !== right.score) {
    return left.score - right.score;
  }
  return left.item.code.toLowerCase().localeCompare(right.item.code.toLowerCase());
};

const collectRankedEntries = (
  availableEmotes: readonly EmoteItem[],
  query: string,
): RankedEntry[] => {
  const ranked: RankedEntry[] = [];
  for (const item of availableEmotes) {
    const score = scoreEmote(item.code, query);
    if (score < NO_MATCH_SCORE) {
      ranked.push({ item, score });
    }
  }
  return ranked;
};

const collectEmoteItems = (entries: readonly RankedEntry[]): EmoteItem[] => {
  const items: EmoteItem[] = [];
  for (const entry of entries) {
    items.push(entry.item);
  }
  return items;
};

export const findActiveEmoteQuery = (
  text: Readonly<string>,
  cursorPos: Readonly<number>,
): ActiveEmoteQuery | null => {
  const beforeCursor = text.slice(ZERO, cursorPos);
  const match = EMOTE_PATTERN.exec(beforeCursor);
  if (match === null) {
    return null;
  }
  const [_fullMatch, _prefix, query] = match;
  return {
    end: beforeCursor.length,
    query,
    start: beforeCursor.length - query.length - ONE,
  };
};

export const rankSuggestions = (
  availableEmotes: readonly EmoteItem[],
  query: Readonly<string>,
  maxSuggestions: Readonly<number>,
): EmoteItem[] => {
  const ranked = collectRankedEntries(availableEmotes, query);
  ranked.sort(compareRankedEntries);
  const limited = ranked.slice(ZERO, maxSuggestions);
  return collectEmoteItems(limited);
};

export const normalizeSingleLine = (value: Readonly<string>): string =>
  value.replaceAll(/[\r\n]+/g, ' ');

export const trimToMaxLength = (value: Readonly<string>, maxLength: Readonly<number>): string =>
  value.slice(ZERO, maxLength);

export const replaceRangeWithEmote = (
  text: Readonly<string>,
  range: Readonly<ActiveEmoteQuery>,
  code: Readonly<string>,
): { cursor: number; text: string } => {
  const before = text.slice(ZERO, range.start);
  const after = text.slice(range.end);
  const nextText = `${before}${code} ${after}`;
  return {
    cursor: range.start + code.length + ONE,
    text: nextText,
  };
};

interface InsertCodeAtCursorArgs {
  readonly code: string;
  readonly cursorPos: number;
  readonly maxLength: number;
  readonly text: string;
}

export const insertCodeAtCursor = (
  args: Readonly<InsertCodeAtCursorArgs>,
): { cursor: number; text: string } => {
  const { code, cursorPos, maxLength, text } = args;
  const before = text.slice(ZERO, cursorPos);
  const after = text.slice(cursorPos);
  const prefix = before.length > ZERO && !before.endsWith(' ') ? ' ' : '';
  const suffix = ' ';
  const nextText = `${before}${prefix}${code}${suffix}${after}`.slice(ZERO, maxLength);
  return {
    cursor: Math.min(before.length + prefix.length + code.length + suffix.length, nextText.length),
    text: nextText,
  };
};
