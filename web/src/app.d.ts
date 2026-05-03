// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }

  // HLS.js type declarations
  class Hls {
    static isSupported(): boolean;
    static Events: {
      MANIFEST_PARSED: string;
      ERROR: string;
    };
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
