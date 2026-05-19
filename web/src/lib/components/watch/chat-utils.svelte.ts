// Chat message parsing utilities
// Extracted from chat.svelte to reduce file size and complexity

export interface ChatPart {
  kind: 'text' | 'emote';
  text?: string;
  id?: string;
  code?: string;
  image_url?: string;
}

export interface ChatMessage {
  id: string;
  kind: 'message' | 'notice';
  sender_display_name: string;
  sender_color: string | null;
  text: string;
  parts: ChatPart[];
}

// Constants
const HEX_RADIX = 16;
const SLICE_START = 2;
const MIN_LENGTH = 0;
const EMPTY_STRING = '';

// Generate unique message ID
const generateMessageId = (): string => {
  const timestamp = Date.now();
  const random = Math.random().toString(HEX_RADIX).slice(SLICE_START);
  return `${timestamp}-${random}`;
};

// Extract image URL from part data
const extractImageUrl = (part: Record<string, unknown>): string | null => {
  const { image_url: imageUrl } = part;
  if (typeof imageUrl === 'string') {
    return imageUrl;
  }
  return null;
};

// Extract sender display name from payload
const extractSenderDisplayName = (payload: Record<string, unknown>): string => {
  const { sender_display_name: displayName } = payload;
  if (typeof displayName === 'string' && displayName.trim().length > MIN_LENGTH) {
    return displayName;
  }

  const { sender_login: senderLogin } = payload;
  if (typeof senderLogin === 'string') {
    return senderLogin;
  }

  return 'system';
};

// Extract sender color from payload
const extractSenderColor = (payload: Record<string, unknown>): string | null => {
  if (payload.kind !== 'message') {
    return null;
  }

  const { sender_color: color } = payload;
  if (typeof color === 'string' && color.trim().length > MIN_LENGTH) {
    return color;
  }

  return null;
};

// Extract message text from payload
const extractMessageText = (payload: Record<string, unknown>): string => {
  const { text: textValue } = payload;
  if (typeof textValue === 'string') {
    return textValue;
  }
  return EMPTY_STRING;
};

// Check if part is text type
const isTextPart = (partRecord: Record<string, unknown>): boolean => {
  return partRecord.kind === 'text' && typeof partRecord.text === 'string';
};

// Check if part is emote type
const isEmotePart = (partRecord: Record<string, unknown>): boolean => {
  return (
    partRecord.kind === 'emote' &&
    typeof partRecord.id === 'string' &&
    typeof partRecord.code === 'string'
  );
};

// Create emote part from data
const createEmotePart = (partRecord: Record<string, unknown>): ChatPart | null => {
  const imageUrlValue = extractImageUrl(partRecord);
  const codeValue = partRecord.code;
  const idValue = partRecord.id;
  if (typeof codeValue !== 'string' || typeof idValue !== 'string') {
    return null;
  }
  return {
    code: codeValue,
    id: idValue,
    image_url: imageUrlValue ?? undefined,
    kind: 'emote',
  };
};

// Process a single part to ChatPart
const processPartToChatPart = (part: unknown): ChatPart | null => {
  if (typeof part !== 'object' || part === null) {
    return null;
  }
  const partRecord = part as Record<string, unknown>;
  if (typeof partRecord.kind !== 'string') {
    return null;
  }

  if (isTextPart(partRecord)) {
    const textValue = partRecord.text;
    if (typeof textValue !== 'string') {
      return null;
    }
    return { kind: 'text', text: textValue };
  }

  if (isEmotePart(partRecord)) {
    return createEmotePart(partRecord);
  }

  return null;
};

// Extract all chat parts from payload
const extractChatParts = (payload: Record<string, unknown>): ChatPart[] => {
  const parts: ChatPart[] = [];
  const { parts: payloadParts } = payload;

  if (!Array.isArray(payloadParts)) {
    return parts;
  }

  for (const part of payloadParts) {
    const chatPart = processPartToChatPart(part);
    if (chatPart !== null) {
      parts.push(chatPart);
    }
  }

  return parts;
};

// Build complete chat message from payload
const buildChatMessage = (payload: Record<string, unknown>): ChatMessage | null => {
  const kindValue = payload.kind;
  if (kindValue !== 'message' && kindValue !== 'notice') {
    return null;
  }
  const senderDisplayName = extractSenderDisplayName(payload);
  const senderColor = extractSenderColor(payload);
  const parts = extractChatParts(payload);
  const text = extractMessageText(payload);
  const id = generateMessageId();

  return {
    id,
    kind: kindValue,
    sender_display_name: senderDisplayName,
    sender_color: senderColor,
    parts,
    text,
  };
};

// Parse raw chat event JSON into ChatMessage
export const parseChatEvent = (raw: string): ChatMessage | null => {
  let payload: unknown;
  try {
    payload = JSON.parse(raw) as unknown;
  } catch {
    return null;
  }

  if (typeof payload !== 'object' || payload === null) {
    return null;
  }
  const payloadRecord = payload as Record<string, unknown>;
  const kindValue = payloadRecord.kind;
  if (kindValue !== 'message' && kindValue !== 'notice') {
    return null;
  }

  return buildChatMessage(payloadRecord);
};

// Generate emote image URL from emote ID
export const emoteUrl = (emoteId: string): string => {
  return `https://static-cdn.jtvnw.net/emoticons/v2/${encodeURIComponent(emoteId)}/default/dark/2.0`;
};

// Format unread message count for display
export const formatUnreadMessage = (count: number): string => {
  const UNREAD_COUNT_ONE = 1;
  const UNREAD_COUNT_MAX = 99;

  if (count <= UNREAD_COUNT_ONE) {
    return '1 new message';
  }
  const displayCount: string = count > UNREAD_COUNT_MAX ? '99+' : String(count);
  return `${displayCount} new messages`;
};
