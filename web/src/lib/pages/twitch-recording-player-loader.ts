const HLS_SCRIPT_PATH = '/static/hls.js';
const RESUME_WAIT_MS = 100;

export const hasHlsLoaded = (): boolean => {
  const hlsValue: unknown = Reflect.get(globalThis, 'Hls');
  if (typeof hlsValue !== 'function') {
    return false;
  }
  return (
    'isSupported' in hlsValue &&
    'Events' in hlsValue &&
    typeof hlsValue.isSupported === 'function' &&
    typeof hlsValue.Events === 'object' &&
    hlsValue.Events !== null
  );
};

const delay = async (timeoutMs: number): Promise<void> => {
  await new Promise<void>((resolve) => {
    globalThis.setTimeout(() => {
      resolve();
    }, timeoutMs);
  });
};

const waitForHlsScript = async (): Promise<boolean> => {
  await delay(RESUME_WAIT_MS);
  return hasHlsLoaded();
};

const loadHlsScript = async (): Promise<boolean> => {
  const script = document.createElement('script');
  script.src = HLS_SCRIPT_PATH;
  script.async = true;
  const loaded = await new Promise<boolean>((resolve) => {
    script.addEventListener('load', () => {
      resolve(true);
    });
    script.addEventListener('error', () => {
      resolve(false);
    });
    document.head.append(script);
  });
  return loaded && hasHlsLoaded();
};

const hasExistingScript = (): boolean =>
  document.querySelector<HTMLScriptElement>(`script[src="${HLS_SCRIPT_PATH}"]`) !== null;

export const ensureHlsLoaded = async (): Promise<boolean> => {
  if (typeof globalThis === 'undefined') {
    return false;
  }
  if (hasHlsLoaded()) {
    return true;
  }

  if (hasExistingScript()) {
    const result = await waitForHlsScript();
    return result;
  }

  const result = await loadHlsScript();
  return result;
};

export const hasPlaylist = async (playlistUrl: string): Promise<boolean> => {
  try {
    const response = await fetch(playlistUrl, { method: 'HEAD' });
    return response.ok;
  } catch {
    return false;
  }
};

const hlsPlaylistUrl = (channelLogin: string, filename: string): string => {
  const params = new URLSearchParams({ channel_login: channelLogin, filename });
  return `/api/recordings/hls-playlist?${params.toString()}`;
};

export const checkPlaylist = async (
  channelLogin: string,
  filename: string,
): Promise<{ exists: true; url: string } | { exists: false }> => {
  const url = hlsPlaylistUrl(channelLogin, filename);
  const exists = await hasPlaylist(url);
  if (exists) {
    return { exists: true, url };
  }
  return { exists: false };
};
