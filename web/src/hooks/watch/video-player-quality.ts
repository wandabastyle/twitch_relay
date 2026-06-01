import {
  AUTO_LEVEL,
  BITRATE_PRECISION,
  FIRST_LEVEL,
  SOURCE_BITRATE_DIVISOR,
  SOURCE_HEIGHT_THRESHOLD,
  SOURCE_INDEX,
  ZERO,
} from './video-player-constants';
import type { HlsInstance, HlsLevel } from './video-player-types';

export const formatBitrate = (bitrate: number): string => {
  if (bitrate <= ZERO) {
    return '';
  }
  return ` (${(bitrate / SOURCE_BITRATE_DIVISOR).toFixed(BITRATE_PRECISION)} Mbps)`;
};

export const qualityLabel = (
  level: Readonly<HlsLevel>,
  idx: number,
  levels: readonly Readonly<HlsLevel>[],
): string => {
  const isFirstLevel = idx === SOURCE_INDEX;
  const hasMultipleLevels = levels.length > FIRST_LEVEL;
  const isHighQuality = level.height >= SOURCE_HEIGHT_THRESHOLD;
  const source = level.name === 'Source' || (isFirstLevel && hasMultipleLevels && isHighQuality);
  return source
    ? `Source${formatBitrate(level.bitrate)}`
    : `${level.height}p${formatBitrate(level.bitrate)}`;
};

export const getQualityDisplay = (
  idx: number,
  hlsLevels: readonly Readonly<HlsLevel>[],
): string | null => {
  if (idx < ZERO || idx >= hlsLevels.length) {
    return null;
  }
  const level = hlsLevels[idx];

  const hasMultipleLevels = hlsLevels.length > FIRST_LEVEL;
  const isHighQuality = level.height >= SOURCE_HEIGHT_THRESHOLD;
  const isFirstLevel = idx === SOURCE_INDEX;
  const isSource = level.name === 'Source' || (isFirstLevel && hasMultipleLevels && isHighQuality);

  if (isSource) {
    return 'Source';
  }
  return `${level.height}p`;
};

export const selectedQualityLabel = (
  qualityLevel: number,
  currentPlayingLevel: number,
  hlsLevels: readonly Readonly<HlsLevel>[],
): string => {
  if (qualityLevel === AUTO_LEVEL) {
    if (currentPlayingLevel >= ZERO && currentPlayingLevel < hlsLevels.length) {
      const level = hlsLevels[currentPlayingLevel];
      return `Auto (${level.height}p)`;
    }
    return 'Auto';
  }

  const display = getQualityDisplay(qualityLevel, hlsLevels);

  if (display === null) {
    return 'Manual';
  }
  return display;
};

export const setQuality = (level: number, hlsInstance: HlsInstance | null): void => {
  if (!hlsInstance) {
    return;
  }
  hlsInstance.currentLevel = level;
};
