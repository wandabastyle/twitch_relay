import { request } from './core.js';

export interface EmoteItem {
  code: string;
  group_key: string;
  group_name: string;
  id: string;
  image_url: string;
}

interface EmotesResponse {
  emotes?: unknown[];
}

const isEmoteItem = (item: unknown): item is EmoteItem => {
  const emote = item as Record<string, unknown>;
  return (
    typeof emote?.id === 'string' &&
    typeof emote?.code === 'string' &&
    typeof emote?.image_url === 'string' &&
    typeof emote?.group_key === 'string' &&
    typeof emote?.group_name === 'string'
  );
};

const normalizeEmoteCode = (code: string): string => code.trim();

/**
 * Fetch and validate chat emotes for a channel.
 * @param channelLogin - The channel login name
 * @returns Array of validated emote items
 */
export const getChatEmotes = async (channelLogin: string): Promise<EmoteItem[]> => {
  if (!channelLogin) {
    return [];
  }

  const MIN_CODE_LENGTH = 1;

  try {
    const response = await request(
      `/api/chat/emotes?channel_login=${encodeURIComponent(channelLogin)}`,
    );

    if (!response.ok) {
      throw new Error('Failed to load emotes');
    }

    const data = (await response.json()) as EmotesResponse;
    const emotes = data.emotes ?? [];

    return emotes
      .filter((item: unknown): item is EmoteItem => isEmoteItem(item))
      .map((item: EmoteItem) => ({
        ...item,
        code: normalizeEmoteCode(item.code),
      }))
      .filter((item: EmoteItem) => item.code.length >= MIN_CODE_LENGTH);
  } catch (error) {
    console.error('Failed to load emotes:', error);
    return [];
  }
};
