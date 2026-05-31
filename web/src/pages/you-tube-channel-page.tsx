import { useCallback, useEffect, useState, type ReactElement } from 'react';
import { SkeletonVideoList, ErrorState, EmptyState, LoadedFade } from '../components/ui';
import { YouTubeVideoRow } from '../components/youtube';
import {
  getYouTubeChannelVideos,
  refreshYouTubeChannelVideos,
  type YoutubeVideo,
} from '../api-client';
import { navigate } from '../router';

interface YouTubeChannelPageProps {
  channel_id: string;
}

const EMPTY_LENGTH = 0;
const DEFAULT_CHANNEL_NAME = 'Channel';
const DEFAULT_SKELETON_COUNT = 6;
const ERROR_NO_ID = 'No channel ID provided';
const ERROR_NO_VIDEOS_DESC = "This channel doesn't have any videos available.";
const ERROR_NO_VIDEOS_TITLE = 'No videos found';
const ERROR_LOAD_FAILED = 'Failed to load videos';

export const YouTubeChannelPage = ({ channel_id }: YouTubeChannelPageProps): ReactElement => {
  const [videos, setVideos] = useState<readonly YoutubeVideo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [channelName, setChannelName] = useState(DEFAULT_CHANNEL_NAME);

  const returnUrl = `/youtube/channel/${channel_id}`;

  const updateChannelName = (channelVideos: readonly YoutubeVideo[]): void => {
    if (channelVideos.length > EMPTY_LENGTH) {
      setChannelName(channelVideos[EMPTY_LENGTH].author);
    }
  };

  const loadInitialChannelVideos = async (): Promise<boolean> => {
    const { fromCache, videos: fetchedVideos } = await getYouTubeChannelVideos(channel_id);
    setVideos(fetchedVideos);
    updateChannelName(fetchedVideos);
    setIsLoading(!fromCache);
    return fromCache;
  };

  const refreshChannelVideos = async (): Promise<void> => {
    const refreshed = await refreshYouTubeChannelVideos(channel_id);
    setVideos(refreshed.videos);
    updateChannelName(refreshed.videos);
  };

  const validateChannelId = (): boolean => {
    if (channel_id) {
      return true;
    }
    setError(ERROR_NO_ID);
    setIsLoading(false);
    return false;
  };

  const startLoading = (): void => {
    setIsLoading(true);
    setError(null);
  };

  const setLoadError = (error_: unknown): void => {
    const errorMessage = error_ instanceof Error ? error_.message : ERROR_LOAD_FAILED;
    setError(errorMessage);
  };

  const loadChannelVideos = async (): Promise<void> => {
    if (!validateChannelId()) {
      return;
    }

    startLoading();

    try {
      const fromCache = await loadInitialChannelVideos();

      // On cache hit, refresh in background; on cold load, data is already fresh
      if (fromCache) {
        await refreshChannelVideos();
      }
    } catch (error_) {
      setLoadError(error_);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    void loadChannelVideos();
  }, [channel_id]);

  const openVideo = useCallback(
    (videoId: string): void => {
      navigate(`/youtube/watch/${encodeURIComponent(videoId)}`, {
        state: { youtubeReturnUrl: returnUrl },
      });
    },
    [returnUrl],
  );

  const goBack = useCallback((): void => {
    navigate('/youtube');
  }, []);

  return (
    <section className="ui-page-panel">
      <header className="panel-header">
        <div className="panel-title">
          <button type="button" className="ui-nav-chip" onClick={goBack}>
            Back
          </button>
          <h1 className="ui-page-title">{channelName}</h1>
          <p className="ui-page-subtle">Latest Videos</p>
        </div>
      </header>

      {isLoading ? (
        <SkeletonVideoList count={DEFAULT_SKELETON_COUNT} />
      ) : (error !== null && error !== '') ? (
        <ErrorState message={error} onRetry={() => { void loadChannelVideos(); }} isRetrying={isLoading} />
      ) : (videos.length === EMPTY_LENGTH) ? (
        <EmptyState
          title={ERROR_NO_VIDEOS_TITLE}
          description={ERROR_NO_VIDEOS_DESC}
          variant="videos"
        />
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
