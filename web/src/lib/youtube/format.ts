// Time conversion constants
const MILLISECONDS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;
const SECONDS_PER_DAY = 86_400;
const DAYS_PER_WEEK = 7;
const DAYS_PER_MONTH = 30;
const DAYS_PER_YEAR = 365;

// Duration formatting constants
const PAD_LENGTH = 2;
const ZERO_THRESHOLD = 0;

// View count formatting constants
const MILLION = 1_000_000;
const THOUSAND = 1000;
const DECIMAL_PLACES = 1;

interface TimeAgoResult {
  readonly days: number;
  readonly hours: number;
  readonly minutes: number;
  readonly months: number;
  readonly seconds: number;
  readonly weeks: number;
  readonly years: number;
}

const calculateTimeAgo = (timestamp: number): TimeAgoResult => {
  const seconds = Math.floor(Date.now() / MILLISECONDS_PER_SECOND) - timestamp;
  const minutes = Math.floor(seconds / SECONDS_PER_MINUTE);
  const hours = Math.floor(seconds / SECONDS_PER_HOUR);
  const days = Math.floor(seconds / SECONDS_PER_DAY);
  const weeks = Math.floor(days / DAYS_PER_WEEK);
  const months = Math.floor(days / DAYS_PER_MONTH);
  const years = Math.floor(days / DAYS_PER_YEAR);

  return { days, hours, minutes, months, seconds, weeks, years };
};

const formatYears = (years: number): string | null =>
  years > ZERO_THRESHOLD ? `${years}y ago` : null;

const formatMonths = (months: number): string | null =>
  months > ZERO_THRESHOLD ? `${months}mo ago` : null;

const formatWeeks = (weeks: number): string | null =>
  weeks > ZERO_THRESHOLD ? `${weeks}w ago` : null;

const formatDays = (days: number): string | null => (days > ZERO_THRESHOLD ? `${days}d ago` : null);

const formatHours = (hours: number): string | null =>
  hours > ZERO_THRESHOLD ? `${hours}h ago` : null;

const formatMinutes = (minutes: number): string | null =>
  minutes > ZERO_THRESHOLD ? `${minutes}m ago` : null;

const buildTimeAgoString = (result: Readonly<TimeAgoResult>): string =>
  formatYears(result.years) ??
  formatMonths(result.months) ??
  formatWeeks(result.weeks) ??
  formatDays(result.days) ??
  formatHours(result.hours) ??
  formatMinutes(result.minutes) ??
  'Just now';

export const formatTimeAgo = (timestamp: number): string => {
  if (!timestamp) {
    return '';
  }
  const timeAgo = calculateTimeAgo(timestamp);
  return buildTimeAgoString(timeAgo);
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
