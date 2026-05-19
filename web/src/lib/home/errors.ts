const MIN_MESSAGE_LENGTH = 1;

export const readJsError = (error: unknown, fallback: string): string => {
  if (error instanceof Error && error.message.trim().length >= MIN_MESSAGE_LENGTH) {
    return error.message;
  }
  return fallback;
};
