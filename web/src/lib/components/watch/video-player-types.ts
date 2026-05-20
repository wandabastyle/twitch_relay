export interface HlsLevel {
  bitrate: number;
  height: number;
  name?: string;
}

export interface HlsInstance {
  currentLevel: number;
  destroy: () => void;
  loadSource: (url: string) => void;
  attachMedia: (element: unknown) => void;
  liveSyncPosition: number | null;
  on: (event: string, callback: (event: string, data: unknown) => void) => void;
  levels: HlsLevel[];
}

export interface HlsStatic {
  new (config: Readonly<Record<string, unknown>>): HlsInstance;
  isSupported: () => boolean;
  Events: { MANIFEST_PARSED: string; LEVEL_SWITCHED: string; ERROR: string };
}
