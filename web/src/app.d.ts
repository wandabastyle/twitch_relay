// HLS.js type declarations
// eslint-disable-next-line no-var
declare var Hls: HlsStatic | undefined;

/**
 * HLS.js player class constructor
 */
interface HlsStatic {
  new (config?: object): HlsInstance;
  isSupported(): boolean;
  Events: {
    MANIFEST_PARSED: string;
    LEVEL_SWITCHED: string;
    ERROR: string;
  };
}

/**
 * HLS.js player instance interface
 */
interface HlsInstance {
  currentLevel: number;
  levels: { height: number; bitrate: number }[];
  liveSyncPosition?: number;
  loadSource(url: string): void;
  attachMedia(media: unknown): void;
  destroy(): void;
  on(event: string, callback: (event: string, data: unknown) => void): void;
  startLoad(position?: number): void;
}
