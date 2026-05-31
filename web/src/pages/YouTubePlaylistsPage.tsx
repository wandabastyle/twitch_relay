import { useCallback, useEffect, useState } from 'react';
import type { ReactElement } from 'react';
import {
  SkeletonMediaList,
  ErrorState,
  EmptyState,
  LoadedFade,
  MediaRow,
  MediaRowMeta,
} from '../components/ui';
import { YouTubeShell } from '../components/youtube';
import { getYouTubePlaylists } from '../lib/api-client';
import type { YoutubePlaylist } from '../lib/api-client';
import { getYouTubePlaylistThumbnailUrl } from '../lib/api-client/youtube-progress';
import { formatTimeAgo } from '../lib/youtube/format';
import { navigate } from '../router';

const FAILED_TO_LOAD = 'Failed to load playlists';
const INITIAL_SLICE_INDEX = 0;
const INITIAL_SLICE_LENGTH = 1;
const NO_PLAYLISTS_DESC = 'Create playlists in YouTube to see them here.';
const NO_PLAYLISTS_TITLE = 'No playlists found';
const SKELETON_COUNT = 6;

export function YouTubePlaylistsPage(): ReactElement {
  const [playlists, setPlaylists] = useState<readonly YoutubePlaylist[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadPlaylists = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await getYouTubePlaylists();
      setPlaylists(data);
    } catch (error_) {
      const errorMessage = error_ instanceof Error ? error_.message : FAILED_TO_LOAD;
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadPlaylists();
  }, [loadPlaylists]);

  const handlePlaylistClick = useCallback((playlistId: string): void => {
    navigate(`/youtube/playlist/${encodeURIComponent(playlistId)}`);
  }, []);

  const getPlaylistInitial = (title: string): string =>
    title.slice(INITIAL_SLICE_INDEX, INITIAL_SLICE_LENGTH).toUpperCase();

  return (
    <YouTubeShell activeTab="playlists" subtitle="Your playlists">
      {isLoading ? (
        <SkeletonMediaList count={SKELETON_COUNT} />
      ) : error !== null && error !== '' ? (
        <ErrorState message={error} onRetry={loadPlaylists} isRetrying={isLoading} />
      ) : playlists.length === 0 ? (
        <EmptyState
          title={NO_PLAYLISTS_TITLE}
          description={NO_PLAYLISTS_DESC}
          variant="playlists"
        />
      ) : (
        <LoadedFade loaded={true}>
          <div className="ui-list">
            {playlists.map((playlist) => (
              <MediaRow
                key={playlist.playlist_id}
                title={playlist.title}
                onClick={() => {
                  handlePlaylistClick(playlist.playlist_id);
                }}
                extraClass="youtube-playlist-row"
                visual={
                  <div className="playlist-thumbnail-container">
                    <img
                      className="ui-thumbnail playlist-thumbnail"
                      src={getYouTubePlaylistThumbnailUrl(playlist.playlist_id)}
                      alt={playlist.title}
                      loading="lazy"
                      onError={(e) => {
                        const img = e.currentTarget as HTMLImageElement;
                        img.style.display = 'none';
                        const fallback = img.nextElementSibling;
                        if (fallback instanceof HTMLElement) {
                          fallback.style.display = 'grid';
                        }
                      }}
                    />
                    <div className="playlist-thumbnail-fallback" style={{ display: 'none' }}>
                      {getPlaylistInitial(playlist.title)}
                    </div>
                  </div>
                }
                meta={
                  <MediaRowMeta>
                    <span className="ui-media-meta playlist-meta">
                      {playlist.video_count} videos · Updated {formatTimeAgo(playlist.updated)}
                    </span>
                  </MediaRowMeta>
                }
              />
            ))}
          </div>
        </LoadedFade>
      )}
    </YouTubeShell>
  );
}
