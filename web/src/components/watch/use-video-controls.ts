import { useCallback, useEffect, useImperativeHandle, useState } from 'react';

export interface VideoControlsHandle {
  enterFullscreen: () => void;
  toggleMute: () => void;
}

interface UseVideoControlsOptions {
  playerRef: React.RefObject<HTMLVideoElement | null>;
  playerHandleRef?: React.RefObject<VideoControlsHandle | null>;
}

interface UseVideoControlsReturn {
  isFullscreen: boolean;
  isPip: boolean;
  toggleFullscreen: () => void;
  togglePip: () => void;
}

const requestFullscreen = async (element: HTMLVideoElement): Promise<void> => {
  try {
    await element.requestFullscreen();
  } catch {
    // Ignore unsupported or rejected fullscreen requests.
  }
};

const exitFullscreen = async (): Promise<void> => {
  try {
    if (document.fullscreenElement) {
      await document.exitFullscreen();
    }
  } catch {
    // Ignore unsupported or rejected fullscreen requests.
  }
};

const requestPip = async (element: HTMLVideoElement): Promise<void> => {
  try {
    await element.requestPictureInPicture();
  } catch {
    // Ignore unsupported or rejected PiP requests.
  }
};

const exitPip = async (): Promise<void> => {
  try {
    if (document.pictureInPictureElement) {
      await document.exitPictureInPicture();
    }
  } catch {
    // Ignore unsupported or rejected PiP requests.
  }
};

export const useVideoControls = (options: UseVideoControlsOptions): UseVideoControlsReturn => {
  const { playerRef, playerHandleRef } = options;
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isPip, setIsPip] = useState(false);

  const enterFullscreen = useCallback((): void => {
    const playerEl = playerRef.current;
    if (!playerEl) {
      return;
    }
    void requestFullscreen(playerEl);
  }, [playerRef]);

  const toggleFullscreen = useCallback((): void => {
    if (document.fullscreenElement) {
      void exitFullscreen();
    } else {
      enterFullscreen();
    }
  }, [enterFullscreen]);

  const enterPip = useCallback((): void => {
    const playerEl = playerRef.current;
    if (!playerEl) {
      return;
    }
    void requestPip(playerEl);
  }, [playerRef]);

  const togglePip = useCallback((): void => {
    if (document.pictureInPictureElement) {
      void exitPip();
    } else {
      enterPip();
    }
  }, [enterPip]);

  const toggleMute = useCallback((): void => {
    const playerEl = playerRef.current;
    if (!playerEl) {
      return;
    }
    playerEl.muted = !playerEl.muted;
  }, [playerRef]);

  useEffect(() => {
    const playerEl = playerRef.current;

    const handleFullscreenChange = (): void => {
      setIsFullscreen(document.fullscreenElement === playerEl);
    };
    const handlePipChange = (): void => {
      setIsPip(document.pictureInPictureElement === playerEl);
    };

    document.addEventListener('fullscreenchange', handleFullscreenChange);
    document.addEventListener('webkitfullscreenchange', handleFullscreenChange);
    playerEl?.addEventListener('enterpictureinpicture', handlePipChange);
    playerEl?.addEventListener('leavepictureinpicture', handlePipChange);

    return (): void => {
      document.removeEventListener('fullscreenchange', handleFullscreenChange);
      document.removeEventListener('webkitfullscreenchange', handleFullscreenChange);
      playerEl?.removeEventListener('enterpictureinpicture', handlePipChange);
      playerEl?.removeEventListener('leavepictureinpicture', handlePipChange);
    };
  }, [playerRef]);

  useImperativeHandle(
    playerHandleRef,
    () => ({
      enterFullscreen,
      toggleMute,
    }),
    [enterFullscreen, toggleMute],
  );

  return {
    isFullscreen,
    isPip,
    toggleFullscreen,
    togglePip,
  };
};
