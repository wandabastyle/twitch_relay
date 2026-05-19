// Time conversion constants
const MILLISECONDS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;
const SECONDS_PER_DAY = 86400;
const DAYS_PER_WEEK = 7;
const DAYS_PER_MONTH = 30;
const DAYS_PER_YEAR = 365;

// Duration formatting constants
const PAD_LENGTH = 2;
const ZERO_THRESHOLD = 0;

// View count formatting constants
const MILLION = 1000000;
const THOUSAND = 1_000;
const DECIMAL_PLACES = 1;

export const formatTimeAgo = (timestamp: number): string => {
  if (!timestamp) {
    return '';
  }
  const seconds = Math.floor(Date.now() / MILLISECONDS_PER_SECOND) - timestamp;
  const minutes = Math.floor(seconds / SECONDS_PER_MINUTE);
  const hours = Math.floor(seconds / SECONDS_PER_HOUR);
  const days = Math.floor(seconds / SECONDS_PER_DAY);
  const weeks = Math.floor(days / DAYS_PER_WEEK);
  const months = Math.floor(days / DAYS_PER_MONTH);
  const years = Math.floor(days / DAYS_PER_YEAR);

  if (years > ZERO_THRESHOLD) {
    return `${years}y ago`;
  }
  if (months > ZERO_THRESHOLD) {
    return `${months}mo ago`;
  }
  if (weeks > ZERO_THRESHOLD) {
    return `${weeks}w ago`;
  }
  if (days > ZERO_THRESHOLD) {
    return `${days}d ago`;
  }
  if (hours > ZERO_THRESHOLD) {
    return `${hours}h ago`;
  }
  if (minutes > ZERO_THRESHOLD) {
    return `${minutes}m ago`;
  }
  return 'Just now';
};

export const formatDuration = (seconds: number): string => {
  const hours = Math.floor(seconds / SECONDS_PER_HOUR);
  const minutes = Math.floor((seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
  const secs = seconds % SECONDS_PER_MINUTE;

  if (hours > ZERO_THRESHOLD) {
    return `${hours}:${minutes.toString().padStart(PAD_LENGTH, '0')}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
};

export const formatViewCount = (count: number): string => {
  if (count >= MILLION) {
    return `${(count / MILLION).toFixed(DECIMAL_PLACES)}M`;
  }
  if (count >= THOUSAND) {
    return `${(count / THOUSAND).toFixed(DECIMAL_PLACES)}K`;
  }
  return String(count);
};
