import { getSelectionRange } from './chat-composer-keyboard';

const ZERO = 0;
const NODE_FILTER_MASK = NodeFilter.SHOW_ELEMENT + NodeFilter.SHOW_TEXT;

export interface GetCursorPositionOptions {
  composerRef: React.RefObject<HTMLDivElement | null>;
  text: string;
}

/**
 * Calculate cursor position in the composer content
 */
export const getCursorPosition = (options: GetCursorPositionOptions): number => {
  const { composerRef, text } = options;
  const { range } = getSelectionRange(composerRef);

  if (range === null) {
    return text.length;
  }

  const composerElement = composerRef.current;
  if (composerElement === null) {
    return text.length;
  }

  // Calculate text length within range
  const preRange = document.createRange();
  preRange.setStart(composerElement, ZERO);
  preRange.setEnd(range.startContainer, range.startOffset);
  const div = document.createElement('div');
  div.append(preRange.cloneContents());

  // Walk nodes and count text length
  let length = ZERO;
  const walker = document.createTreeWalker(div, NODE_FILTER_MASK, null);
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
};
