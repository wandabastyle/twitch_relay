const hlsPlaylistUrl = (channelLogin: string, filename: string): string => {
  const params = new URLSearchParams({ channel_login: channelLogin, filename });
  return `/api/recordings/hls-playlist?${params.toString()}`;
};

const hasPlaylist = async (playlistUrl: string): Promise<boolean> => {
  try {
    const response = await fetch(playlistUrl, { method: 'HEAD' });
    return response.ok;
  } catch {
    return false;
  }
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
