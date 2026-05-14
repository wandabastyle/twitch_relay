import { request } from "./core";

export interface EmoteItem {
  id: string;
  code: string;
  image_url: string;
  group_key: string;
  group_name: string;
}

interface EmotesResponse {
  emotes?: unknown[];
}

function isEmoteItem(item: unknown): item is EmoteItem {
  const obj = item as Record<string, unknown>;
  return (
    typeof obj?.id === "string" &&
    typeof obj?.code === "string" &&
    typeof obj?.image_url === "string" &&
    typeof obj?.group_key === "string" &&
    typeof obj?.group_name === "string"
  );
}

function normalizeEmoteCode(code: string): string {
  return code.trim();
}

/**
 * Fetch and validate chat emotes for a channel.
 * @param channelLogin - The channel login name
 * @returns Array of validated emote items
 */
export async function getChatEmotes(channelLogin: string): Promise<EmoteItem[]> {
  if (!channelLogin) {
    return [];
  }

  try {
    const response = await request(
      `/api/chat/emotes?channel_login=${encodeURIComponent(channelLogin)}`,
    );

    if (!response.ok) {
      throw new Error("Failed to load emotes");
    }

    const data = (await response.json()) as EmotesResponse;
    const emotes = data.emotes ?? [];

    return emotes
      .filter((item: unknown): item is EmoteItem => isEmoteItem(item))
      .map((item: EmoteItem) => ({
        ...item,
        code: normalizeEmoteCode(item.code),
      }))
      .filter((item: EmoteItem) => item.code.length > 0);
  } catch (error) {
    console.error("Failed to load emotes:", error);
    return [];
  }
}
