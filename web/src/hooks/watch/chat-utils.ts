// Chat message parsing utilities

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

type JsonRecord = Readonly<Record<string, unknown>>;

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
const toJsonRecord = (value: unknown): JsonRecord | null => {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  // Build record from object entries - Object.entries returns [string, unknown][]
  const record: Record<string, unknown> = {};
  const entries = Object.entries(value);
  for (const [key, val] of entries) {
    record[key] = val;
  }
  return record;
};

const extractImageUrl = (part: JsonRecord): string | null => {
  const { image_url: imageUrl } = part;
  if (typeof imageUrl === 'string') {
    return imageUrl;
  }
  return null;
};

// Extract sender display name from payload
const extractSenderDisplayName = (payload: JsonRecord): string => {
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
const extractSenderColor = (payload: JsonRecord): string | null => {
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
const extractMessageText = (payload: JsonRecord): string => {
  const { text: textValue } = payload;
  if (typeof textValue === 'string') {
    return textValue;
  }
  return EMPTY_STRING;
};

// Check if part is text type
const isTextPart = (partRecord: JsonRecord): boolean =>
  partRecord.kind === 'text' && typeof partRecord.text === 'string';

// Check if part is emote type
const isEmotePart = (partRecord: JsonRecord): boolean =>
  partRecord.kind === 'emote' &&
  typeof partRecord.id === 'string' &&
  typeof partRecord.code === 'string';

// Create emote part from data
const createEmotePart = (partRecord: JsonRecord): ChatPart | null => {
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
const createTextPart = (partRecord: JsonRecord): ChatPart | null => {
  const textValue = partRecord.text;
  if (typeof textValue !== 'string') {
    return null;
  }
  return { kind: 'text', text: textValue };
};

const processPartToChatPart = (part: unknown): ChatPart | null => {
  const partRecord = toJsonRecord(part);
  if (partRecord === null) {
    return null;
  }

  if (typeof partRecord.kind !== 'string') {
    return null;
  }

  if (isTextPart(partRecord)) {
    return createTextPart(partRecord);
  }

  if (isEmotePart(partRecord)) {
    return createEmotePart(partRecord);
  }

  return null;
};

// Extract all chat parts from payload
const extractChatParts = (payload: JsonRecord): ChatPart[] => {
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
const buildChatMessage = (payload: JsonRecord): ChatMessage | null => {
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
    parts,
    sender_color: senderColor,
    sender_display_name: senderDisplayName,
    text,
  };
};

// Parse raw chat event JSON into ChatMessage
export const parseChatEvent = (raw: Readonly<string>): ChatMessage | null => {
  let payload: unknown = null;
  try {
    payload = JSON.parse(raw);
  } catch {
    // Payload remains null on error
  }

  const payloadRecord = toJsonRecord(payload);
  if (payloadRecord === null) {
    return null;
  }

  const kindValue = payloadRecord.kind;
  if (kindValue !== 'message' && kindValue !== 'notice') {
    return null;
  }

  return buildChatMessage(payloadRecord);
};

// Generate emote image URL from emote ID
export const emoteUrl = (emoteId: string): string =>
  `https://static-cdn.jtvnw.net/emoticons/v2/${encodeURIComponent(emoteId)}/default/dark/2.0`;

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
