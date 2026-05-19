// HLS.js type declarations
declare global {
  class Hls {
    static isSupported(): boolean;
    static Events: {
      MANIFEST_PARSED: string;
      LEVEL_SWITCHED: string;
      ERROR: string;
    };
    currentLevel: number;
    levels: Array<{ height: number; bitrate: number }>;
    liveSyncPosition?: number;
    constructor(config?: object);
    loadSource(url: string): void;
    attachMedia(media: HTMLVideoElement): void;
    destroy(): void;
    on(event: string, callback: (event: string, data: unknown) => void): void;
  }

  interface Window {
    Hls: typeof Hls;
  }
}

export {};
