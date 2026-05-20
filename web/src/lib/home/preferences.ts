const LIVE_ONLY_PREF_KEY = 'twitchRelay.liveOnly';
const LIVE_ONLY_ENABLED = '1';
const LIVE_ONLY_DISABLED = '0';

const getPreferenceValue = (enabled: boolean): string => {
  if (enabled) {
    return LIVE_ONLY_ENABLED;
  }
  return LIVE_ONLY_DISABLED;
};

export const loadLiveOnlyPreference = (): boolean => {
  try {
    return globalThis.window.localStorage.getItem(LIVE_ONLY_PREF_KEY) === LIVE_ONLY_ENABLED;
  } catch {
    return false;
  }
};

export const saveLiveOnlyPreference = (value: boolean): void => {
  try {
    globalThis.window.localStorage.setItem(LIVE_ONLY_PREF_KEY, getPreferenceValue(value));
  } catch {
    // Ignore storage failures and keep in-memory state
  }
};
