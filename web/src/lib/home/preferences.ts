export const LIVE_ONLY_PREF_KEY = 'twitchRelay.liveOnly';

export function loadLiveOnlyPreference(): boolean {
  try {
    return window.localStorage.getItem(LIVE_ONLY_PREF_KEY) === '1';
  } catch {
    return false;
  }
}

export function saveLiveOnlyPreference(value: boolean): void {
  try {
    window.localStorage.setItem(LIVE_ONLY_PREF_KEY, value ? '1' : '0');
  } catch {
    // Ignore storage failures and keep in-memory state
  }
}
