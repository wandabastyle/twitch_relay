const PREVIEW_SIZE = 112;
const PREVIEW_OFFSET = 8;
const CENTER_DIVISOR = 2;

export interface CalculatePreviewPositionOptions {
  rect: DOMRect;
}

export interface PreviewPosition {
  left: number;
  top: number;
}

export const calculatePreviewPosition = (
  options: CalculatePreviewPositionOptions,
): PreviewPosition => {
  const { rect } = options;
  return {
    left: rect.left + rect.width / CENTER_DIVISOR - PREVIEW_SIZE / CENTER_DIVISOR,
    top: rect.top - PREVIEW_SIZE - PREVIEW_OFFSET,
  };
};

export interface ClearPreviewOptions {
  setPreviewOpen: (open: boolean) => void;
  previewTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>;
}

export const clearPreview = (options: ClearPreviewOptions): void => {
  const { setPreviewOpen, previewTimerRef } = options;
  setPreviewOpen(false);
  if (previewTimerRef.current !== null) {
    clearTimeout(previewTimerRef.current);
    previewTimerRef.current = null;
  }
};

export interface StartPreviewTimerOptions {
  rect: DOMRect;
  imageUrl: string;
  previewDelayMs: number;
  previewTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>;
  setPreviewUrl: (url: string) => void;
  setPreviewPosition: (position: PreviewPosition) => void;
  setPreviewOpen: (open: boolean) => void;
}

export const startPreviewTimer = (options: StartPreviewTimerOptions): void => {
  const {
    rect,
    imageUrl,
    previewDelayMs,
    previewTimerRef,
    setPreviewUrl,
    setPreviewPosition,
    setPreviewOpen,
  } = options;

  // Clear any existing timer
  if (previewTimerRef.current !== null) {
    clearTimeout(previewTimerRef.current);
  }
  // Set new timer
  previewTimerRef.current = setTimeout(() => {
    setPreviewUrl(imageUrl);
    setPreviewPosition(calculatePreviewPosition({ rect }));
    setPreviewOpen(true);
  }, previewDelayMs);
};

export interface EndPreviewOptions {
  previewTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>;
  setPreviewOpen: (open: boolean) => void;
}

export const endPreview = (options: EndPreviewOptions): void => {
  const { previewTimerRef, setPreviewOpen } = options;
  if (previewTimerRef.current !== null) {
    clearTimeout(previewTimerRef.current);
    previewTimerRef.current = null;
  }
  setPreviewOpen(false);
};
