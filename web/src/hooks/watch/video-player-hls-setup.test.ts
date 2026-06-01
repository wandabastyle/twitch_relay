import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AUTO_LEVEL, DEFAULT_LEVEL } from './video-player-constants';
import {
  createHandleHlsError,
  createHandleLevelSwitched,
  createHandleManifestParsed,
} from './video-player-hls-setup';
import type { HlsLevel } from './video-player-types';

const SWITCHED_LEVEL = 2;
const BITRATE_720P = 2_000_000;
const BITRATE_1080P = 4_000_000;
const BITRATE_480P = 1_500_000;
const BITRATE_360P = 3_000_000;
const HEIGHT_360 = 360;
const HEIGHT_480 = 480;
const HEIGHT_720 = 720;
const HEIGHT_1080 = 1080;

interface MockHlsInstance {
  attachMedia: () => void;
  currentLevel: number;
  destroy: () => void;
  levels: unknown[];
  liveSyncPosition: null;
  loadSource: () => void;
  on: () => void;
}

const createMockHlsInstance = (levels: unknown[] = []): MockHlsInstance => ({
  attachMedia: vi.fn<() => void>(),
  currentLevel: AUTO_LEVEL,
  destroy: vi.fn<() => void>(),
  levels,
  liveSyncPosition: null,
  loadSource: vi.fn<() => void>(),
  on: vi.fn<() => void>(),
});

// Simulates hls.js Level class instances
class HlsLevelClass {
  public bitrate: number;
  public height: number;
  public name?: string;
  public constructor(bitrate: number, height: number, name?: string) {
    this.bitrate = bitrate;
    this.height = height;
    if (name !== undefined) {
      this.name = name;
    }
  }
}

describe('createHandleManifestParsed', () => {
  // eslint-disable-next-line init-declarations -- initialized in beforeEach
  let setHlsLevelsMock: (levels: HlsLevel[]) => void;

  beforeEach(() => {
    setHlsLevelsMock = vi.fn<(levels: HlsLevel[]) => void>();
  });

  it('extracts levels from event data when available', () => {
    const mockLevels: HlsLevel[] = [
      { bitrate: BITRATE_720P, height: HEIGHT_720 },
      { bitrate: BITRATE_1080P, height: HEIGHT_1080 },
    ];
    const mockInstance = createMockHlsInstance([]);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    handler('MANIFEST_PARSED', { levels: mockLevels });

    expect(setHlsLevelsMock).toHaveBeenCalledWith(mockLevels);
  });

  it('falls back to hlsInstance.levels when event data has no levels', () => {
    const mockLevels: HlsLevel[] = [
      { bitrate: BITRATE_480P, height: HEIGHT_480 },
      { bitrate: BITRATE_360P, height: HEIGHT_720 },
    ];
    const mockInstance = createMockHlsInstance(mockLevels);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    handler('MANIFEST_PARSED', {});

    expect(setHlsLevelsMock).toHaveBeenCalledWith(mockLevels);
  });

  it('filters out levels with non-numeric height or bitrate', () => {
    const mockInstance = createMockHlsInstance([]);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    const invalidLevels = [
      { bitrate: BITRATE_720P, height: HEIGHT_720 },
      { bitrate: 'invalid', height: HEIGHT_480 },
      { bitrate: BITRATE_1080P, height: 'invalid' },
      { bitrate: null, height: null },
      { name: 'Test' },
    ];

    handler('MANIFEST_PARSED', { levels: invalidLevels });

    expect(setHlsLevelsMock).toHaveBeenCalledWith([{ bitrate: BITRATE_720P, height: HEIGHT_720 }]);
  });

  it('extracts levels from class instances with height and bitrate properties', () => {
    const mockInstance = createMockHlsInstance([]);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    const classInstanceLevels = [
      new HlsLevelClass(BITRATE_720P, HEIGHT_720, '720p'),
      new HlsLevelClass(BITRATE_1080P, HEIGHT_1080),
      // Invalid entries that should be filtered out
      { bitrate: 'invalid', height: HEIGHT_480 },
      null,
      undefined,
    ];

    handler('MANIFEST_PARSED', { levels: classInstanceLevels });

    expect(setHlsLevelsMock).toHaveBeenCalledWith([
      { bitrate: BITRATE_720P, height: HEIGHT_720, name: '720p' },
      { bitrate: BITRATE_1080P, height: HEIGHT_1080 },
    ]);
  });

  it('filters out levels with non-numeric height or bitrate from hlsInstance.levels fallback', () => {
    const mockLevels = [
      { bitrate: BITRATE_720P, height: HEIGHT_720 },
      { bitrate: 'invalid', height: HEIGHT_480 },
      { bitrate: BITRATE_1080P, height: 'invalid' },
      { bitrate: null, height: null },
      { name: 'Test' },
    ];
    const mockInstance = createMockHlsInstance(mockLevels);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    handler('MANIFEST_PARSED', {});

    expect(setHlsLevelsMock).toHaveBeenCalledWith([{ bitrate: BITRATE_720P, height: HEIGHT_720 }]);
  });

  it('extracts levels from class instances in hlsInstance.levels fallback', () => {
    const classInstanceLevels = [
      new HlsLevelClass(BITRATE_480P, HEIGHT_480),
      new HlsLevelClass(BITRATE_360P, HEIGHT_720, '720p'),
      // Invalid entry that should be filtered out
      { bitrate: 'invalid', height: HEIGHT_360 },
    ];
    const mockInstance = createMockHlsInstance(classInstanceLevels);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    handler('MANIFEST_PARSED', {});

    expect(setHlsLevelsMock).toHaveBeenCalledWith([
      { bitrate: BITRATE_480P, height: HEIGHT_480 },
      { bitrate: BITRATE_360P, height: HEIGHT_720, name: '720p' },
    ]);
  });

  it('sets empty array when event data levels is not an array and hlsInstance.levels is empty', () => {
    const mockInstance = createMockHlsInstance([]);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    handler('MANIFEST_PARSED', { levels: null });

    expect(setHlsLevelsMock).toHaveBeenCalledWith([]);
  });

  it('sets empty array when event data and hlsInstance.levels both have no valid levels', () => {
    const invalidLevels: Record<string, unknown>[] = [{ name: 'invalid' }];
    const mockInstance = createMockHlsInstance(invalidLevels);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    handler('MANIFEST_PARSED', { levels: [{ name: 'invalid' }] });

    expect(setHlsLevelsMock).toHaveBeenCalledWith([]);
  });

  it('includes name property when present', () => {
    const mockLevels: HlsLevel[] = [
      { bitrate: BITRATE_720P, height: HEIGHT_720, name: '720p' },
      { bitrate: BITRATE_1080P, height: HEIGHT_1080 },
    ];
    const mockInstance = createMockHlsInstance([]);
    const handler = createHandleManifestParsed(
      setHlsLevelsMock,
      // @ts-expect-error -- mocking HlsInstance for testing
      mockInstance,
    );

    handler('MANIFEST_PARSED', { levels: mockLevels });

    expect(setHlsLevelsMock).toHaveBeenCalledWith(mockLevels);
  });
});

describe('createHandleLevelSwitched', () => {
  it('sets current playing level from event data', () => {
    const setCurrentPlayingLevelMock = vi.fn<(level: number) => void>();
    const setQualityLevelMock = vi.fn<(level: number) => void>();
    const handler = createHandleLevelSwitched(
      setCurrentPlayingLevelMock,
      setQualityLevelMock,
      false,
    );

    handler('LEVEL_SWITCHED', { level: SWITCHED_LEVEL });

    expect(setCurrentPlayingLevelMock).toHaveBeenCalledWith(SWITCHED_LEVEL);
  });

  it('sets quality level to auto when user selected auto', () => {
    const setCurrentPlayingLevelMock = vi.fn<(level: number) => void>();
    const setQualityLevelMock = vi.fn<(level: number) => void>();
    const handler = createHandleLevelSwitched(
      setCurrentPlayingLevelMock,
      setQualityLevelMock,
      true,
    );

    handler('LEVEL_SWITCHED', { level: 1 });

    expect(setQualityLevelMock).toHaveBeenCalledWith(AUTO_LEVEL);
  });

  it('uses default level when parsed level is not a number', () => {
    const setCurrentPlayingLevelMock = vi.fn<(level: number) => void>();
    const setQualityLevelMock = vi.fn<(level: number) => void>();
    const handler = createHandleLevelSwitched(
      setCurrentPlayingLevelMock,
      setQualityLevelMock,
      false,
    );

    handler('LEVEL_SWITCHED', { level: 'invalid' });

    expect(setCurrentPlayingLevelMock).toHaveBeenCalledWith(DEFAULT_LEVEL);
  });

  it('uses default level when event data is null', () => {
    const setCurrentPlayingLevelMock = vi.fn<(level: number) => void>();
    const setQualityLevelMock = vi.fn<(level: number) => void>();
    const handler = createHandleLevelSwitched(
      setCurrentPlayingLevelMock,
      setQualityLevelMock,
      false,
    );

    handler('LEVEL_SWITCHED', null);

    expect(setCurrentPlayingLevelMock).toHaveBeenCalledWith(DEFAULT_LEVEL);
  });
});

describe('createHandleHlsError', () => {
  it('calls onError with message for fatal errors', () => {
    const onErrorMock = vi.fn<(message: string) => void>();
    const handler = createHandleHlsError(onErrorMock);

    handler('ERROR', { fatal: true });

    expect(onErrorMock).toHaveBeenCalledWith(
      'Stream unavailable. The channel may be offline or not accessible.',
    );
  });

  it('does not call onError for non-fatal errors', () => {
    const onErrorMock = vi.fn<(message: string) => void>();
    const handler = createHandleHlsError(onErrorMock);

    handler('ERROR', { fatal: false });

    expect(onErrorMock).not.toHaveBeenCalled();
  });

  it('does not call onError when fatal is undefined', () => {
    const onErrorMock = vi.fn<(message: string) => void>();
    const handler = createHandleHlsError(onErrorMock);

    handler('ERROR', { type: 'network' });

    expect(onErrorMock).not.toHaveBeenCalled();
  });
});
