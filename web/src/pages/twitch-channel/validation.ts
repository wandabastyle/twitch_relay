const NOT_WHOLE_NUMBER_ERROR = 'must be a whole number';
const MIN_VALUE_ERROR = 'must be at least 1';
const MIN_VALUE = 1;

export const normalizeValue = (value: string | number | undefined): string => {
  if (value === undefined) {
    return '';
  }
  if (typeof value === 'number') {
    return String(value);
  }
  return value;
};

export const parseOptionalPositiveInt = (
  value: string | number | undefined,
  label: string,
): number | undefined => {
  const normalized = normalizeValue(value);
  const trimmed = normalized.trim();

  if (!trimmed) {
    return undefined;
  }

  if (!/^\d+$/.test(trimmed)) {
    throw new Error(`${label} ${NOT_WHOLE_NUMBER_ERROR}`);
  }

  const parsed = Number(trimmed);
  if (!Number.isSafeInteger(parsed) || parsed < MIN_VALUE) {
    throw new Error(`${label} ${MIN_VALUE_ERROR}`);
  }

  return parsed;
};
