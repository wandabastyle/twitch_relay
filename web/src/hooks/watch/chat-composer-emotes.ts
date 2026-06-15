import type { EmoteChip } from './use-chat-composer';

const NBSP = '\u00A0';
const ZERO = 0;
const ONE = 1;
const BR_TAG = 'br';
const NEWLINE_CHAR = '\n';
const SPACE_CHAR = ' ';

export interface CreateEmoteImageElementOptions {
  code: string;
  imageUrl: string;
  onPreviewStart: (rect: DOMRect, imageUrl: string) => void;
  onPreviewEnd: () => void;
  previewDelayMs: number;
}

export const createEmoteImageElement = (
  options: CreateEmoteImageElementOptions,
): HTMLSpanElement => {
  const { code, imageUrl, onPreviewStart, onPreviewEnd } = options;
  const wrapper = document.createElement('span');
  wrapper.className = 'ui-chat-composer-emote-wrap';
  wrapper.contentEditable = 'false';

  const img = document.createElement('img');
  img.className = 'ui-chat-composer-emote';
  img.dataset.code = code;
  img.dataset.imageUrl = imageUrl;
  img.src = imageUrl;
  img.alt = code;
  img.draggable = false;

  // Add hover preview handlers
  wrapper.addEventListener('mouseenter', () => {
    const rect = wrapper.getBoundingClientRect();
    onPreviewStart(rect, imageUrl);
  });

  wrapper.addEventListener('mouseleave', () => {
    onPreviewEnd();
  });

  wrapper.append(img);
  return wrapper;
};

export interface RenderComposerContentOptions {
  composerElement: HTMLDivElement | null;
  textValue: string;
  chips: EmoteChip[];
  createEmoteElement: (code: string, imageUrl: string) => HTMLSpanElement;
}

const appendTextWithNewlines = (element: HTMLDivElement, text: string): void => {
  const lines = text.split(NEWLINE_CHAR);
  for (let index = ZERO; index < lines.length; index += ONE) {
    // Use NBSP to make trailing spaces visible in contenteditable
    element.append(document.createTextNode(lines[index].replaceAll(SPACE_CHAR, NBSP)));
    // Add <br> for all but the last line
    if (index < lines.length - ONE) {
      element.append(document.createElement(BR_TAG));
    }
  }
};

export const renderComposerContent = (options: RenderComposerContentOptions): void => {
  const { composerElement, textValue, chips, createEmoteElement } = options;
  if (composerElement === null) {
    return;
  }

  // Sort chips by position
  const sortedChips = [...chips].sort((first, second) => first.position - second.position);

  // Clear existing content
  composerElement.textContent = '';

  let currentPos = ZERO;
  for (const chip of sortedChips) {
    // Add text before this chip, handling newlines
    const textBefore = textValue.slice(currentPos, chip.position);
    if (textBefore.length > ZERO) {
      appendTextWithNewlines(composerElement, textBefore);
    }

    // Add the emote image
    const img = createEmoteElement(chip.code, chip.image_url);
    composerElement.append(img);

    // Move past the emote code in the text
    currentPos = chip.position + chip.code.length;
  }

  // Add remaining text, handling newlines
  const textAfter = textValue.slice(currentPos);
  if (textAfter.length > ZERO) {
    appendTextWithNewlines(composerElement, textAfter);
  }
};

export interface ReadComposerModelResult {
  text: string;
  chips: EmoteChip[];
}

const extractEmoteFromNode = (node: HTMLSpanElement): { code: string; imageUrl: string } | null => {
  const img = node.querySelector('img.ui-chat-composer-emote');
  if (!(img instanceof HTMLImageElement)) {
    return null;
  }
  const { code, imageUrl } = img.dataset;
  if (code === undefined || code === '' || imageUrl === undefined || imageUrl === '') {
    return null;
  }
  return { code, imageUrl };
};

export const readComposerModel = (
  composerElement: HTMLDivElement | null,
): ReadComposerModelResult => {
  if (composerElement === null) {
    return { chips: [], text: '' };
  }

  let resultText = '';
  const chips: EmoteChip[] = [];

  for (const node of composerElement.childNodes) {
    if (node.nodeType === Node.TEXT_NODE) {
      // Convert NBSP back to regular spaces for canonical text
      const textContent = node.textContent ?? '';
      resultText += textContent.replaceAll(NBSP, SPACE_CHAR);
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      if (
        node instanceof HTMLSpanElement &&
        node.classList.contains('ui-chat-composer-emote-wrap')
      ) {
        const emoteData = extractEmoteFromNode(node);
        if (emoteData !== null) {
          resultText += emoteData.code;
          chips.push({
            code: emoteData.code,
            image_url: emoteData.imageUrl,
            position: resultText.length - emoteData.code.length,
          });
        }
      } else if (node instanceof HTMLBRElement) {
        // Convert <br> to newline character
        resultText += NEWLINE_CHAR;
      }
    }
  }

  return { chips, text: resultText };
};
