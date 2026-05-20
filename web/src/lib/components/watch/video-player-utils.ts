import {
  BITRATE_PRECISION,
  FIRST_LEVEL,
  SOURCE_BITRATE_DIVISOR,
  SOURCE_HEIGHT_THRESHOLD,
  SOURCE_INDEX,
  ZERO,
} from '$lib/components/watch/video-player-constants';
import type { HlsInstance, HlsLevel, HlsStatic } from '$lib/components/watch/video-player-types';

export type { HlsInstance, HlsLevel, HlsStatic } from '$lib/components/watch/video-player-types';
export {
  AUTO_LEVEL,
  DEFAULT_LEVEL,
  FIRST_LEVEL,
  HLS_PATH,
  SOURCE_HEIGHT_THRESHOLD,
  SOURCE_INDEX,
} from '$lib/components/watch/video-player-constants';
export {
  getQualityDisplay,
  qualityLabel,
  selectedQualityLabel,
  setQuality,
} from '$lib/components/watch/video-player-quality';
export { ensureHlsLoaded } from '$lib/components/watch/video-player-hls-loader';

const getPrototypeSafely = (value: object): unknown => Object.getPrototypeOf(value);

const isPlainObject = (value: unknown): value is Record<string, unknown> => {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const prototype = getPrototypeSafely(value);
  return prototype === Object.prototype || prototype === null;
};

export const toObject = (value: unknown): Record<string, unknown> | null =>
  isPlainObject(value) ? value : null;

export const formatBitrate = (bitrate: number): string =>
  bitrate <= ZERO ? '' : ` (${(bitrate / SOURCE_BITRATE_DIVISOR).toFixed(BITRATE_PRECISION)} Mbps)`;

const hasHlsShape = (value: unknown): value is HlsStatic => {
  if (typeof value !== 'function') {
    return false;
  }
  const hasIsSupported = 'isSupported' in value && typeof value.isSupported === 'function';
  const hasEvents = 'Events' in value && typeof value.Events === 'object' && value.Events !== null;
  return hasIsSupported && hasEvents;
};

export const getHlsClass = (): HlsStatic | null => {
  const hlsValue: unknown = Reflect.get(globalThis, 'Hls');
  if (!hasHlsShape(hlsValue)) {
    return null;
  }
  return hlsValue;
};
