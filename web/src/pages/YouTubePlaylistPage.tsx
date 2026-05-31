import { useCallback, useEffect, useState, type ReactElement } from 'react';
import { SkeletonVideoList, ErrorState, EmptyState, LoadedFade } from '../components/ui';
import { YouTubeVideoRow } from '../components/youtube';
import { getYouTubePlaylistVideos, refreshYouTubePlaylistVideos, type YoutubeVideo } from '../api-client';
import { navigate } from '../router';

interface YouTubePlaylistPageProps {
  playlist_id: string;
}

const DEFAULT_ERROR_MESSAGE = 'Failed to load playlist videos';
const MIN_VIDEOS_FOR_TITLE = 1;
const NO_ID_ERROR = 'No playlist ID provided';
const NO_VIDEOS_DESC = "This playlist doesn't contain any videos.";
const NO_VIDEOS_TITLE = 'No videos found';
const PLAYLIST_TITLE = 'Playlist';
const PLAYLIST_VIDEOS_TITLE = 'Playlist Videos';
const VIDEO_COUNT_IN_SKELETON = 6;

export const YouTubePlaylistPage = ({ playlist_id }: YouTubePlaylistPageProps): ReactElement => {
  const [videos, setVideos] = useState<readonly YoutubeVideo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [playlistTitle, setPlaylistTitle] = useState(PLAYLIST_TITLE);

  const returnUrl = `/youtube/playlist/${playlist_id}`;

  const updatePlaylistTitle = (loadedVideos: readonly YoutubeVideo[]): void => {
    if (loadedVideos.length >= MIN_VIDEOS_FOR_TITLE) {
      setPlaylistTitle(PLAYLIST_VIDEOS_TITLE);
    }
  };

  const loadInitialPlaylist = async (): Promise<boolean> => {
    const { fromCache, videos: loadedVideos } = await getYouTubePlaylistVideos(playlist_id);
    setVideos(loadedVideos);
    updatePlaylistTitle(loadedVideos);
    return fromCache;
  };

  const refreshPlaylist = async (): Promise<void> => {
    const refreshed = await refreshYouTubePlaylistVideos(playlist_id);
    setVideos(refreshed.videos);
    updatePlaylistTitle(refreshed.videos);
  };

  const setLoadError = (catchError: unknown): void => {
    const errorMessage = catchError instanceof Error ? catchError.message : DEFAULT_ERROR_MESSAGE;
    setError(errorMessage);
  };

  const loadPlaylistVideos = async (): Promise<void> => {
    if (!playlist_id) {
      setError(NO_ID_ERROR);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const fromCache = await loadInitialPlaylist();

      // On cache hit, refresh in background; on cold load, data is already fresh
      if (fromCache) {
        await refreshPlaylist();
      }
    } catch (catchError) {
      setLoadError(catchError);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    void loadPlaylistVideos();
  }, [playlist_id]);

  const openVideo = useCallback(
    (videoId: string): void => {
      navigate(`/youtube/watch/${encodeURIComponent(videoId)}`, {
        state: { youtubeReturnUrl: returnUrl },
      });
    },
    [returnUrl],
  );

  const goBack = useCallback((): void => {
    navigate('/youtube/playlists');
  }, []);

  return (
    <section className="ui-page-panel">
      <header className="panel-header">
        <div className="panel-title">
          <button type="button" className="ui-nav-chip" onClick={goBack}>
            Back
          </button>
          <h1>{playlistTitle}</h1>
          <p className="header-subtle">{videos.length} videos</p>
        </div>
      </header>

      {isLoading ? (
        <SkeletonVideoList count={VIDEO_COUNT_IN_SKELETON} />
      ) : error !== null && error !== '' ? (
        <ErrorState message={error} onRetry={loadPlaylistVideos} isRetrying={isLoading} />
      ) : videos.length === 0 ? (
        <EmptyState title={NO_VIDEOS_TITLE} description={NO_VIDEOS_DESC} variant="videos" />
      ) : (
        <LoadedFade loaded={true}>
          <div className="youtube-video-list">
            {videos.map((video) => (
              <YouTubeVideoRow
                key={video.video_id}
                video={video}
                onClick={() => {
                  openVideo(video.video_id);
                }}
              />
            ))}
          </div>
        </LoadedFade>
      )}
    </section>
  );
}
