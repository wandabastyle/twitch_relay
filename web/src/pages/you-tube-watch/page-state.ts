import { useSyncExternalStore } from 'react';
import type { HistoryState } from '../../router';

const INITIAL_RAW_STATE = Symbol('initial raw history state');

let cachedRawState: unknown = INITIAL_RAW_STATE;
let cachedSnapshot: HistoryState | null = null;

const readPageState = (): HistoryState | null => {
  const rawState: unknown = globalThis.history.state;
  if (Object.is(rawState, cachedRawState)) {
    return cachedSnapshot;
  }

  cachedRawState = rawState;
  if (typeof rawState !== 'object' || rawState === null) {
    cachedSnapshot = null;
    return cachedSnapshot;
  }

  const descriptor = Object.getOwnPropertyDescriptor(rawState, 'youtubeReturnUrl');
  if (descriptor !== undefined && typeof descriptor.value === 'string') {
    cachedSnapshot = { youtubeReturnUrl: descriptor.value };
    return cachedSnapshot;
  }

  cachedSnapshot = null;
  return cachedSnapshot;
};

export const usePageState = (): HistoryState | null =>
  useSyncExternalStore(
    (callback): (() => void) => {
      globalThis.addEventListener('popstate', callback);
      return (): void => {
        globalThis.removeEventListener('popstate', callback);
      };
    },
    readPageState,
    () => null,
  );
