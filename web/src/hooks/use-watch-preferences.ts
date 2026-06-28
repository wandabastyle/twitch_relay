import { type Dispatch, type SetStateAction, useEffect, useState } from 'react';

const THEATER_STORAGE_KEY = 'twitch-relay-watch-theater';
const CHAT_COLLAPSED_STORAGE_KEY = 'twitch-relay-watch-chat-collapsed';

export const useWatchPreferences = (): {
  isChatCollapsed: boolean;
  setIsChatCollapsed: Dispatch<SetStateAction<boolean>>;
  theaterMode: boolean;
  setTheaterMode: Dispatch<SetStateAction<boolean>>;
} => {
  const [theaterMode, setTheaterMode] = useState(false);
  const [isChatCollapsed, setIsChatCollapsed] = useState(false);

  useEffect(() => {
    const savedTheater = globalThis.localStorage.getItem(THEATER_STORAGE_KEY);
    setTheaterMode(savedTheater === 'true');

    const savedCollapsed = globalThis.localStorage.getItem(CHAT_COLLAPSED_STORAGE_KEY);
    setIsChatCollapsed(savedCollapsed === 'true');
  }, []);

  useEffect(() => {
    globalThis.localStorage.setItem(THEATER_STORAGE_KEY, String(theaterMode));
  }, [theaterMode]);

  useEffect(() => {
    globalThis.localStorage.setItem(CHAT_COLLAPSED_STORAGE_KEY, String(isChatCollapsed));
  }, [isChatCollapsed]);

  useEffect(() => {
    if (theaterMode) {
      document.body.dataset.theater = 'true';
    } else {
      delete document.body.dataset.theater;
    }
  }, [theaterMode]);

  return { isChatCollapsed, setIsChatCollapsed, setTheaterMode, theaterMode };
};
