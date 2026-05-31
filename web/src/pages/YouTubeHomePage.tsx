import { useCallback, useEffect, useState, type ReactElement } from 'react';
import {
  SkeletonMediaList,
  ErrorState,
  EmptyState,
  LoadedFade,
  MediaRow,
  MediaRowMeta,
} from '../components/ui';
import { YouTubeShell } from '../components/youtube';
import { getYouTubeSubscriptions, type YoutubeChannel } from '../api-client';
import { navigate } from '../router';

const EMPTY_LENGTH = 0;
const FAILED_TO_LOAD = 'Failed to load subscriptions';
const LIST_ITEM_COUNT = 8;
const NO_SUBS_DESC = 'Subscribe to YouTube channels in Invidious to see them here.';
const NO_SUBS_TITLE = 'No subscriptions found';
const INITIAL_SLICE_INDEX = 0;
const INITIAL_SLICE_LENGTH = 1;

export const YouTubeHomePage = (): ReactElement => {
  const [channels, setChannels] = useState<readonly YoutubeChannel[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadSubscriptions = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await getYouTubeSubscriptions();
      setChannels(data);
    } catch (error_) {
      const errorMessage = error_ instanceof Error ? error_.message : FAILED_TO_LOAD;
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadSubscriptions();
  }, [loadSubscriptions]);

  const openChannel = useCallback((channelId: string): void => {
    navigate(`/youtube/channel/${encodeURIComponent(channelId)}`);
  }, []);

  return (
    <YouTubeShell activeTab="subscriptions">
      {isLoading ? (
        <SkeletonMediaList count={LIST_ITEM_COUNT} />
      ) : error !== null && error !== '' ? (
        <ErrorState message={error} onRetry={() => { void loadSubscriptions(); }} isRetrying={isLoading} />
      ) : channels.length === EMPTY_LENGTH ? (
        <EmptyState title={NO_SUBS_TITLE} description={NO_SUBS_DESC} variant="channels" />
      ) : (
        <LoadedFade loaded={true}>
          <div className="ui-list">
            {channels.map((channel) => (
              <MediaRow
                key={channel.channel_id}
                title={channel.name}
                onClick={() => {
                  openChannel(channel.channel_id);
                }}
                extraClass="youtube-channel-row"
                visual={
                  channel.avatar !== undefined && channel.avatar !== '' ? (
                    <img
                      className="ui-avatar channel-avatar"
                      src={channel.avatar}
                      alt={channel.name}
                      loading="lazy"
                    />
                  ) : (
                    <div className="ui-avatar ui-avatar-fallback channel-avatar fallback">
                      {channel.name.slice(INITIAL_SLICE_INDEX, INITIAL_SLICE_LENGTH)}
                    </div>
                  )
                }
                meta={
                  channel.description !== undefined && channel.description !== '' ? (
                    <MediaRowMeta>
                      <span
                        className="ui-media-meta channel-description"
                        title={channel.description}
                      >
                        {channel.description}
                      </span>
                    </MediaRowMeta>
                  ) : undefined
                }
              />
            ))}
          </div>
        </LoadedFade>
      )}
    </YouTubeShell>
  );
}
