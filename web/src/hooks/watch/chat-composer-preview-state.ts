import { useCallback, useRef, useState } from 'react';
import {
  startPreviewTimer,
  endPreview,
  clearPreview as clearPreviewBase,
  type PreviewPosition,
} from './chat-composer-preview';
import {
  createEmoteImageElement as createEmoteImageElementBase,
  type CreateEmoteImageElementOptions,
} from './chat-composer-emotes';

const ZERO = 0;

export interface UseComposerPreviewReturn {
  previewOpen: boolean;
  previewUrl: string;
  previewPosition: { left: number; top: number };
  previewTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>;
  createEmoteImageElement: (code: string, imageUrl: string) => HTMLSpanElement;
  clearPreview: () => void;
  getEmoteImageUrl: (code: string, availableEmotes: { code: string; image_url: string }[]) => string | null;
}

export const useComposerPreview = (
  _getEmoteImageUrlFn: (code: string) => string | null,
): UseComposerPreviewReturn => {
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewUrl, setPreviewUrl] = useState('');
  const [previewPosition, setPreviewPosition] = useState<PreviewPosition>({ left: ZERO, top: ZERO });

  const PREVIEW_DELAY_MS = 350;

  const createEmoteImageElement = useCallback(
    (code: string, imageUrl: string): HTMLSpanElement => {
      const emoteElementOptions: CreateEmoteImageElementOptions = {
        code,
        imageUrl,
        onPreviewEnd: () => {
          endPreview({ previewTimerRef, setPreviewOpen });
        },
        onPreviewStart: (rect, imgUrl) => {
          startPreviewTimer({
            imageUrl: imgUrl,
            previewDelayMs: PREVIEW_DELAY_MS,
            previewTimerRef,
            rect,
            setPreviewOpen,
            setPreviewPosition,
            setPreviewUrl,
          });
        },
        previewDelayMs: PREVIEW_DELAY_MS,
      };
      return createEmoteImageElementBase(emoteElementOptions);
    },
    [PREVIEW_DELAY_MS],
  );

  const clearPreview = useCallback((): void => {
    clearPreviewBase({ previewTimerRef, setPreviewOpen });
  }, []);

  const getEmoteImageUrl = useCallback(
    (code: string, availableEmotes: { code: string; image_url: string }[]): string | null => {
      const emote = availableEmotes.find((item) => item.code === code);
      return emote?.image_url ?? null;
    },
    [],
  );

  return {
    clearPreview,
    createEmoteImageElement,
    getEmoteImageUrl,
    previewOpen,
    previewPosition,
    previewTimerRef,
    previewUrl,
  };
};
