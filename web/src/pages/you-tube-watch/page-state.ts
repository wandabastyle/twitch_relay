import { useSyncExternalStore } from 'react';
import type { HistoryState } from '../../router';

export const usePageState = (): HistoryState | null => {
  return useSyncExternalStore(
    (callback) => {
      window.addEventListener('popstate', callback);
      return () => {
        window.removeEventListener('popstate', callback);
      };
    },
    () => {
      const rawState: unknown = window.history.state;
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
};
