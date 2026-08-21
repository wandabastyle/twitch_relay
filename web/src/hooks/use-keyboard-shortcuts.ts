import { type Dispatch, type SetStateAction, useCallback, useEffect } from 'react';

interface KeyboardShortcutsOptions {
  enabled?: boolean;
  onFocusChat: () => void;
  onFullscreen: () => void;
  onMute: () => void;
  onTheater: () => void;
  theaterMode: boolean;
}

const isTypingTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return (
    target.isContentEditable ||
    target.closest('.ui-chat-composer') !== null ||
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA'
  );
};

export const useKeyboardShortcuts = (options: KeyboardShortcutsOptions): void => {
  const { enabled = true, onFocusChat, onFullscreen, onMute, onTheater, theaterMode } = options;

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const handleKeydown = (event: KeyboardEvent): void => {
      if (isTypingTarget(event.target)) {
        return;
      }

      switch (event.key) {
        case 'f':
        case 'F': {
          event.preventDefault();
          onFullscreen();
          break;
        }
        case 't':
        case 'T': {
          event.preventDefault();
          onTheater();
          break;
        }
        case 'm':
        case 'M': {
          event.preventDefault();
          onMute();
          break;
        }
        case 'c':
        case 'C': {
          event.preventDefault();
          onFocusChat();
          break;
        }
        case 'Escape': {
          if (theaterMode) {
            event.preventDefault();
            onTheater();
          }
          break;
        }
        default: {
          break;
        }
      }
    };

    document.addEventListener('keydown', handleKeydown);
    return (): void => {
      document.removeEventListener('keydown', handleKeydown);
    };
  }, [enabled, onFocusChat, onFullscreen, onMute, onTheater, theaterMode]);
};

export const useToggleCallback = (setter: Dispatch<SetStateAction<boolean>>): (() => void) =>
  useCallback((): void => {
    setter((prev: boolean) => !prev);
  }, [setter]);
