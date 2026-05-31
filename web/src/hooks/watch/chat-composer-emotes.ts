import type { EmoteChip } from './useChatComposer';

const NBSP = '\u00A0';
const ZERO = 0;

export interface CreateEmoteImageElementOptions {
  code: string;
  imageUrl: string;
  onPreviewStart: (rect: DOMRect, imageUrl: string) => void;
  onPreviewEnd: () => void;
  previewDelayMs: number;
}

export function createEmoteImageElement(
  options: CreateEmoteImageElementOptions,
): HTMLSpanElement {
  const { code, imageUrl, onPreviewStart, onPreviewEnd, previewDelayMs } = options;
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
}

export interface RenderComposerContentOptions {
  composerElement: HTMLDivElement | null;
  textValue: string;
  chips: EmoteChip[];
  createEmoteElement: (code: string, imageUrl: string) => HTMLSpanElement;
}

export function renderComposerContent(options: RenderComposerContentOptions): void {
  const { composerElement, textValue, chips, createEmoteElement } = options;
  if (composerElement === null) {
    return;
  }

  // Sort chips by position
  const sortedChips = [...chips].sort((first, second) => first.position - second.position);

  // Clear existing content
  composerElement.textContent = '';

  let currentPos = 0;
  for (const chip of sortedChips) {
    // Add text before this chip
    const textBefore = textValue.slice(currentPos, chip.position);
    if (textBefore.length > ZERO) {
      // Use NBSP to make trailing spaces visible in contenteditable
      composerElement.append(document.createTextNode(textBefore.replaceAll(' ', NBSP)));
    }

    // Add the emote image
    const img = createEmoteElement(chip.code, chip.image_url);
    composerElement.append(img);

    // Move past the emote code in the text
    currentPos = chip.position + chip.code.length;
  }

  // Add remaining text
  const textAfter = textValue.slice(currentPos);
  if (textAfter.length > ZERO) {
    // Use NBSP to make trailing spaces visible in contenteditable
    composerElement.append(document.createTextNode(textAfter.replaceAll(' ', NBSP)));
  }
}

export interface ReadComposerModelResult {
  text: string;
  chips: EmoteChip[];
}

export function readComposerModel(
  composerElement: HTMLDivElement | null,
): ReadComposerModelResult {
  if (composerElement === null) {
    return { chips: [], text: '' };
  }

  let resultText = '';
  const chips: EmoteChip[] = [];

  for (const node of composerElement.childNodes) {
    if (node.nodeType === Node.TEXT_NODE) {
      // Convert NBSP back to regular spaces for canonical text
      const textContent = node.textContent ?? '';
      resultText += textContent.replaceAll(NBSP, ' ');
    } else if (
      node.nodeType === Node.ELEMENT_NODE &&
      node instanceof HTMLSpanElement &&
      node.classList.contains('ui-chat-composer-emote-wrap')
    ) {
      const img = node.querySelector('img.ui-chat-composer-emote') as HTMLImageElement | null;
      if (img !== null) {
        const { code } = img.dataset;
        const { imageUrl } = img.dataset;
        if (code !== undefined && code !== '' && imageUrl !== undefined && imageUrl !== '') {
          resultText += code;
          chips.push({
            code,
            image_url: imageUrl,
            position: resultText.length - code.length,
          });
        }
      }
    }
  }

  return { chips, text: resultText };
}
