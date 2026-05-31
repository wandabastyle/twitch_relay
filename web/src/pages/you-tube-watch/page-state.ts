import { useSyncExternalStore } from 'react';
import type { HistoryState } from '../../router';

export const usePageState = (): HistoryState | null =>
  useSyncExternalStore(
    (callback) => {
      globalThis.addEventListener('popstate', callback);
      return () => {
        globalThis.removeEventListener('popstate', callback);
      };
    },
    () => {
      const rawState: unknown = globalThis.history.state;
      if (typeof rawState !== 'object' || rawState === null) {
        return null;
      }
      const descriptor = Object.getOwnPropertyDescriptor(rawState, 'youtubeReturnUrl');
      if (descriptor !== undefined && typeof descriptor.value === 'string') {
        return { youtubeReturnUrl: descriptor.value };
      }
      return null;
    },
    () => null,
  );
