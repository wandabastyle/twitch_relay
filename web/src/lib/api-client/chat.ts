import { isObject, request } from './core.js';

export interface EmoteItem {
  code: string;
  group_key: string;
  group_name: string;
  id: string;
  image_url: string;
}

const isEmoteItem = (item: unknown): item is EmoteItem => {
  if (!isObject(item)) {
    return false;
  }
  return (
    typeof item.id === 'string' &&
    typeof item.code === 'string' &&
    typeof item.image_url === 'string' &&
    typeof item.group_key === 'string' &&
    typeof item.group_name === 'string'
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

    const rawData: unknown = await response.json();
    if (!isObject(rawData) || !Array.isArray(rawData.emotes)) {
      return [];
    }
    const emotes: unknown[] = rawData.emotes;

    return emotes
      .filter((item: unknown): item is EmoteItem => isEmoteItem(item))
      .map((item: Readonly<EmoteItem>) => ({
        ...item,
        code: normalizeEmoteCode(item.code),
      }))
      .filter((item: Readonly<EmoteItem>) => item.code.length >= MIN_CODE_LENGTH);
  } catch (error) {
    console.error('Failed to load emotes:', error);
    return [];
  }
};
