import { writable } from "svelte/store";
import { browser } from "$app/environment";

export type RelayMode = "twitch" | "youtube";

function createModeStore() {
  const { subscribe, set } = writable<RelayMode>("twitch");

  return {
    subscribe,
    toggle: () => {
      const current = getCurrentMode();
      const newMode = current === "twitch" ? "youtube" : "twitch";
      set(newMode);
      if (browser) {
        document.body.setAttribute("data-theme", newMode === "youtube" ? "youtube" : "");
        localStorage.setItem("relayMode", newMode);
      }
    },
    init: () => {
      if (browser) {
        const saved = localStorage.getItem("relayMode") as RelayMode;
        if (saved === "youtube") {
          set("youtube");
          document.body.setAttribute("data-theme", "youtube");
        }
      }
    },
    setTwitch: () => {
      set("twitch");
      if (browser) {
        document.body.setAttribute("data-theme", "");
        localStorage.setItem("relayMode", "twitch");
      }
    },
    setYoutube: () => {
      set("youtube");
      if (browser) {
        document.body.setAttribute("data-theme", "youtube");
        localStorage.setItem("relayMode", "youtube");
      }
    },
  };
}

function getCurrentMode(): RelayMode {
  if (!browser) return "twitch";
  return (localStorage.getItem("relayMode") as RelayMode) || "twitch";
}

export const relayMode = createModeStore();
