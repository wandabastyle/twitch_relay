// HLS.js type declarations
declare global {
  class Hls {
    public static isSupported(): boolean;
    public static Events: {
      MANIFEST_PARSED: string;
      LEVEL_SWITCHED: string;
      ERROR: string;
    };
    public currentLevel: number;
    public levels: { height: number; bitrate: number }[];
    public liveSyncPosition?: number;
    public constructor(config?: object);
    public loadSource(url: string): void;
    public attachMedia(media: Readonly<HTMLVideoElement>): void;
    public destroy(): void;
    public on(event: string, callback: (event: string, data: unknown) => void): void;
  }

  interface Window {
    Hls: typeof Hls;
  }
}

export type {};
