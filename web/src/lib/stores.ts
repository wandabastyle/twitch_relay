import { writable } from "svelte/store";
import { browser } from "$app/environment";

export type RelayMode = "twitch" | "youtube";

function createModeStore() {
  const { subscribe, set, update } = writable<RelayMode>("twitch");

  function applyMode(mode: RelayMode): void {
    if (!browser) return;
    document.body.setAttribute("data-theme", mode === "youtube" ? "youtube" : "");
    localStorage.setItem("relayMode", mode);
  }

  return {
    subscribe,
    toggle: () => {
      update((current) => {
        const newMode = current === "twitch" ? "youtube" : "twitch";
        applyMode(newMode);
        return newMode;
      });
    },
    init: () => {
      set("twitch");
      applyMode("twitch");
    },
    setTwitch: () => {
      set("twitch");
      applyMode("twitch");
    },
    setYoutube: () => {
      set("youtube");
      applyMode("youtube");
    },
  };
}

export const relayMode = createModeStore();
