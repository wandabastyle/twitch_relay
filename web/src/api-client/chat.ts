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
const MIN_CODE_LENGTH = 1;

const toValidEmote = (item: unknown): EmoteItem | null => {
  if (!isEmoteItem(item)) {
    return null;
  }
  const code = normalizeEmoteCode(item.code);
  if (code.length < MIN_CODE_LENGTH) {
    return null;
  }
  return {
    code,
    group_key: item.group_key,
    group_name: item.group_name,
    id: item.id,
    image_url: item.image_url,
  };
};

const parseEmotesPayload = (rawData: unknown): EmoteItem[] => {
  if (!isObject(rawData) || !Array.isArray(rawData.emotes)) {
    return [];
  }

  const result: EmoteItem[] = [];
  for (const item of rawData.emotes) {
    const emote = toValidEmote(item);
    if (emote !== null) {
      result.push(emote);
    }
  }
  return result;
};

/**
 * Fetch and validate chat emotes for a channel.
 * @param channelLogin - The channel login name
 * @returns Array of validated emote items
 */
export const getChatEmotes = async (channelLogin: string): Promise<EmoteItem[]> => {
  if (!channelLogin) {
    return [];
  }

  try {
    const response = await request(
      `/api/chat/emotes?channel_login=${encodeURIComponent(channelLogin)}`,
    );
    if (!response.ok) {
      return [];
    }

    return parseEmotesPayload(await response.json());
  } catch {
    return [];
  }
};
