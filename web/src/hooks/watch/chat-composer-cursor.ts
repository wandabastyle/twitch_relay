import type { EmoteChip } from './use-chat-composer';

const ZERO = 0;
const ONE = 1;
const HALF = 2;

export interface WalkState {
  currentPos: number;
  targetNode: Node | null;
  targetOffset: number;
}

/**
 * Walk through nodes to find cursor position
 */
export const walkToCursorTarget = (node: Node, position: number, state: WalkState): boolean => {
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
    const img = node.querySelector('img.ui-chat-composer-emote');
    if (img instanceof HTMLImageElement) {
      const { code } = img.dataset;
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
  }

  // Walk child nodes for other element types
  return [...node.childNodes].some((child) => walkToCursorTarget(child, position, state));
};

/**
 * Apply cursor selection to a node
 */
export const applyCursorSelection = (selection: Selection, node: Node, offset: number): void => {
  const range = document.createRange();

  // Handle element nodes (e.g., composer.el itself when placing cursor before/after images)
  if (node.nodeType === Node.ELEMENT_NODE && node instanceof Element) {
    const childCount = node.childNodes.length;
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

export interface GetRangeTextLengthOptions {
  composerElement: HTMLDivElement | null;
  range: Range;
  fallbackLength: number;
}

/**
 * Calculate text length within a range, accounting for emote images
 */
export const getRangeTextLength = (options: GetRangeTextLengthOptions): number => {
  const { composerElement, range, fallbackLength } = options;

  if (composerElement === null) {
    return fallbackLength;
  }
  const preRange = document.createRange();
  preRange.setStart(composerElement, ZERO);
  preRange.setEnd(range.startContainer, range.startOffset);
  const div = document.createElement('div');
  div.append(preRange.cloneContents());

  // Walk nodes and count text length
  // For emote images, count data-code.length instead of 0 (textContent returns 0 for images)
  let length = ZERO;
  // Use numeric addition instead of bitwise OR to satisfy lint rules
  const nodeFilter = NodeFilter.SHOW_ELEMENT + NodeFilter.SHOW_TEXT;
  const walker = document.createTreeWalker(div, nodeFilter, null);
  while (walker.nextNode()) {
    const walkerNode = walker.currentNode;
    if (walkerNode.nodeType === Node.TEXT_NODE) {
      length += walkerNode.textContent?.length ?? ZERO;
    } else if (
      walkerNode.nodeType === Node.ELEMENT_NODE &&
      walkerNode instanceof HTMLImageElement
    ) {
      const { code } = walkerNode.dataset;
      if (code !== undefined && code !== '') {
        length += code.length;
      }
    }
  }
  return length;
};

export interface InsertEmoteChipOptions {
  safeCode: string;
  imageUrl: string;
  cursorPos: number;
  newEmotePosition: number;
  emoteChips: EmoteChip[];
  lengthDiff: number;
}

/**
 * Create new chips array with inserted emote and adjusted positions
 */
export const insertEmoteChip = (options: InsertEmoteChipOptions): EmoteChip[] => {
  const { safeCode, imageUrl, cursorPos, newEmotePosition, emoteChips, lengthDiff } = options;

  const newChips: EmoteChip[] = [];

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

  return newChips;
};

export interface SetCursorPositionOptions {
  composerElement: HTMLDivElement | null;
  position: number;
}

/**
 * Set cursor position in the composer
 */
export const setCursorPositionBase = (options: SetCursorPositionOptions): void => {
  const { composerElement, position } = options;

  if (composerElement === null) {
    return;
  }
  const selection = globalThis.getSelection();
  if (selection === null) {
    return;
  }
  const state: WalkState = { currentPos: ZERO, targetNode: null, targetOffset: ZERO };
  walkToCursorTarget(composerElement, position, state);
  if (state.targetNode === null) {
    return;
  }
  applyCursorSelection(selection, state.targetNode, state.targetOffset);
};
