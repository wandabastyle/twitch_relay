export type InitialHomeView =
  | { relayMode: "twitch"; twitchView: "channels" | "recordings" }
  | { relayMode: "youtube"; youtubeView: "subscriptions" | "recent" | "playlists" };

export function parseInitialHomeView(search: string): InitialHomeView {
  const params = new URLSearchParams(search);
  const twitchView = params.get("twitch");
  const youtubeView = params.get("youtube");

  if (twitchView === "recordings") {
    return { relayMode: "twitch", twitchView: "recordings" };
  } else if (twitchView === "channels") {
    return { relayMode: "twitch", twitchView: "channels" };
  } else if (youtubeView === "subscriptions") {
    return { relayMode: "youtube", youtubeView: "subscriptions" };
  } else if (youtubeView === "recent") {
    return { relayMode: "youtube", youtubeView: "recent" };
  } else if (youtubeView === "playlists") {
    return { relayMode: "youtube", youtubeView: "playlists" };
  } else {
    return { relayMode: "twitch", twitchView: "channels" };
  }
}
