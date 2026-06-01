export const getEmbeddedVideoElement = (
  frame: HTMLIFrameElement | null,
): HTMLVideoElement | null => {
  if (frame === null) {
    return null;
  }
  try {
    const frameWindow = frame.contentWindow;
    if (frameWindow === null) {
      return null;
    }
    return frameWindow.document.querySelector('video');
  } catch {
    return null;
  }
};
