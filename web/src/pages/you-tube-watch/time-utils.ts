const HOURS_IN_SECONDS = 3600;
const MINUTES_IN_SECONDS = 60;
const PAD_LENGTH = 2;
const ZERO = 0;

const DEFAULT_END_GAP = 20;
const PERCENTAGE_MULTIPLIER = 0.05;

export const formatDuration = (seconds: number): string => {
  const hours = Math.floor(seconds / HOURS_IN_SECONDS);
  const minutes = Math.floor((seconds % HOURS_IN_SECONDS) / MINUTES_IN_SECONDS);
  const secs = seconds % MINUTES_IN_SECONDS;

  if (hours > ZERO) {
    return `${hours}:${minutes.toString().padStart(PAD_LENGTH, '0')}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(PAD_LENGTH, '0')}`;
};

export const getEndGapSecs = (duration: number): number =>
  !Number.isFinite(duration) || duration <= ZERO
    ? DEFAULT_END_GAP
    : Math.min(DEFAULT_END_GAP, duration * PERCENTAGE_MULTIPLIER);
