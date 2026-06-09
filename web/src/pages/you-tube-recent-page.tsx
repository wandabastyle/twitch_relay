import { useCallback, useEffect, useRef, useState, type ReactElement } from 'react';
import { getYouTubeRecentVideos, type YoutubeVideo } from '../api-client';
import { getYouTubeThumbnailUrl } from '../api-client/youtube-progress';
import {
  SkeletonVideoList,
  ErrorState,
  EmptyState,
  LoadedFade,
  MediaRow,
  MediaRowMeta,
} from '../components/ui';
import { YouTubeShell } from '../components/youtube';
import { navigate } from '../router';
import { formatDuration, formatTimeAgo, formatViewCount } from '../utils/youtube-format';

const DEFAULT_MAX_RESULTS = 25;
const EMPTY_LENGTH = 0;
const FAILED_TO_LOAD = 'Failed to load recent videos';
const NO_VIDEOS_DESC = 'Recent videos from your subscriptions will appear here.';
const NO_VIDEOS_TITLE = 'No recent videos found';
const RETURN_URL = '/youtube/recent';
const SKELETON_COUNT = 6;
const INITIAL_REQUEST_ID = 0;
const REQUEST_ID_INCREMENT = 1;

export const YouTubeRecentPage = (): ReactElement => {
  const [videos, setVideos] = useState<readonly YoutubeVideo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const latestRequestIdRef = useRef(INITIAL_REQUEST_ID);

  const loadRecentVideos = useCallback(async (): Promise<void> => {
    latestRequestIdRef.current += REQUEST_ID_INCREMENT;
    const requestId = latestRequestIdRef.current;

    setIsLoading(true);
    setError(null);
    try {
      const data = await getYouTubeRecentVideos(DEFAULT_MAX_RESULTS);

      if (requestId !== latestRequestIdRef.current) {
        return;
      }

      setVideos(data);
    } catch (error_) {
      if (requestId !== latestRequestIdRef.current) {
        return;
      }

      const errorMessage = error_ instanceof Error ? error_.message : FAILED_TO_LOAD;
      setError(errorMessage);
    } finally {
      if (requestId === latestRequestIdRef.current) {
        setIsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void loadRecentVideos();
  }, [loadRecentVideos]);

  const openVideo = useCallback((videoId: string): void => {
    navigate(`/youtube/watch/${encodeURIComponent(videoId)}`, {
      state: { youtubeReturnUrl: RETURN_URL },
    });
  }, []);

  const renderRecentVideos = (): ReactElement => {
    if (isLoading) {
      return <SkeletonVideoList count={SKELETON_COUNT} />;
    }
    if (error !== null && error !== '') {
      return (
        <ErrorState
          message={error}
          onRetry={() => {
            void loadRecentVideos();
          }}
          isRetrying={isLoading}
        />
      );
    }
    if (videos.length === EMPTY_LENGTH) {
      return <EmptyState title={NO_VIDEOS_TITLE} description={NO_VIDEOS_DESC} variant="videos" />;
    }
    return (
      <LoadedFade loaded={true}>
        <div className="ui-list">
          {videos.map((video) => (
            <MediaRow
              key={video.video_id}
              title={video.title}
              onClick={() => {
                openVideo(video.video_id);
              }}
              extraClass="youtube-recent-row"
              visual={
                <div className="recent-thumbnail-wrap">
                  <img
                    className="ui-thumbnail recent-thumbnail"
                    src={getYouTubeThumbnailUrl(video.video_id)}
                    alt={video.title}
                    loading="lazy"
                  />
                  <span className="recent-duration">{formatDuration(video.duration)}</span>
                </div>
              }
              meta={
                <MediaRowMeta>
                  <span className="ui-media-meta recent-meta">
                    {video.author} · {formatViewCount(video.view_count)} ·{' '}
                    {formatTimeAgo(video.published)}
                  </span>
                </MediaRowMeta>
              }
            />
          ))}
        </div>
      </LoadedFade>
    );
  };

  return (
    <YouTubeShell activeTab="recent" subtitle="Recent videos from subscriptions">
      {renderRecentVideos()}
    </YouTubeShell>
  );
};
