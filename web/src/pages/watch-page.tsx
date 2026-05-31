import { useCallback, useEffect, useState, type ReactElement } from 'react';
import { Chat } from '../components/watch/chat';
import { VideoPlayer } from '../components/watch/video-player';
import { useRouter } from '../hooks/use-router';
import {
  getWatchSession,
  getTwitchStatus,
  getChatEmotes,
  getTwitchConnectUrl,
  type EmoteItem,
} from '../api-client';

const EMPTY_MESSAGE_LENGTH = 0;
const ERROR_MISSING_TICKET = 'Missing watch ticket.';
const ERROR_SESSION_FAILED = 'Failed to initialize watch session.';
const RELAY_PARAM = '1';

interface ChatStatus {
  available: boolean;
  connected: boolean;
  message: string;
}

export const WatchPage = (): ReactElement => {
  const { page, navigate } = useRouter();
  const { ticket } = page.params;

  const [channelLogin, setChannelLogin] = useState('');
  const [appVersion, setAppVersion] = useState('');
  const [manifestUrl, setManifestUrl] = useState('');
  const [watchLoading, setWatchLoading] = useState(true);
  const [watchError, setWatchError] = useState<string | undefined>();
  const [playbackError, setPlaybackError] = useState<string | undefined>();
  const [chatAvailable, setChatAvailable] = useState(false);
  const [availableEmotes, setAvailableEmotes] = useState<EmoteItem[]>([]);
  const [twitchStatusChecked, setTwitchStatusChecked] = useState(false);

  const connectTwitchUrl = getTwitchConnectUrl();

  const readMessage = useCallback((err: unknown, fallback: string): string => {
    if (err instanceof Error && err.message.trim().length > EMPTY_MESSAGE_LENGTH) {
      return err.message;
    }
    return fallback;
  }, []);

  const isRelayForced = useCallback((): boolean => {
    if (typeof globalThis === 'undefined') {
      return false;
    }
    return new URLSearchParams(globalThis.location.search).get('relay') === RELAY_PARAM;
  }, []);

  const loadEmotes = useCallback(async (channel: string): Promise<void> => {
    if (!channel) {
      return;
    }
    const emotes = await getChatEmotes(channel);
    setAvailableEmotes(emotes);
  }, []);

  const setupChat = useCallback(
    async (channel: string): Promise<void> => {
      setChatAvailable(false);
      setTwitchStatusChecked(false);

      try {
        const twitchStatus = await getTwitchStatus();
        setTwitchStatusChecked(true);
        if (!twitchStatus.connected) {
          setChatAvailable(false);
          return;
        }

        setChatAvailable(true);
        await loadEmotes(channel);
      } catch {
        setChatAvailable(false);
        setTwitchStatusChecked(true);
      }
    },
    [loadEmotes],
  );

  const initializeWatchPage = useCallback(async (): Promise<void> => {
    setWatchLoading(true);
    setWatchError(undefined);
    setPlaybackError(undefined);

    const forceRelay = isRelayForced();
    try {
      const session = await getWatchSession(ticket, forceRelay);
      setChannelLogin(session.channel);
      setAppVersion(session.app_version);
      setManifestUrl(session.manifest_url);

      setWatchLoading(false);

      // Setup chat
      await setupChat(session.channel);
    } catch (error) {
      setWatchError(readMessage(error, ERROR_SESSION_FAILED));
      setWatchLoading(false);
    }
  }, [ticket, isRelayForced, readMessage, setupChat]);

  const handleChatStatusChange = useCallback((status: ChatStatus): void => {
    setChatAvailable(status.available);
  }, []);

  const handlePlaybackError = useCallback((msg: string): void => {
    setPlaybackError(msg);
  }, []);

  const handleBackToChannels = useCallback((): void => {
    navigate('/twitch');
  }, [navigate]);

  const handleConnectTwitch = useCallback((): void => {
    globalThis.location.href = connectTwitchUrl;
  }, [connectTwitchUrl]);

  useEffect(() => {
    if (ticket === '') {
      setWatchError(ERROR_MISSING_TICKET);
      setWatchLoading(false);
      return;
    }

    void initializeWatchPage();
  }, [ticket, initializeWatchPage]);

  return (
    <section className="watch-page">
      <header className="watch-page-header">
        <div className="watch-page-meta">
          <strong>{channelLogin || 'stream'}</strong>
          <span>via Twitch Relay{appVersion ? ` · v${appVersion}` : ''}</span>
        </div>
        <div className="watch-page-actions">
          <button type="button" className="ui-nav-chip" onClick={handleBackToChannels}>
            Back to channels
          </button>
          {twitchStatusChecked && !chatAvailable && (
            <button type="button" className="ui-nav-chip" onClick={handleConnectTwitch}>
              Connect Twitch
            </button>
          )}
        </div>
      </header>

      {watchLoading ? (
        <div className="watch-loading-state">
          <p className="ui-muted">Loading watch session...</p>
        </div>
      ) : (watchError !== undefined && watchError !== '') ? (
        <div className="watch-loading-state">
          <p className="ui-error">{watchError}</p>
        </div>
      ) : (
        <div className="watch-layout">
          <section className="watch-player-panel">
            <VideoPlayer manifestUrl={manifestUrl} onError={handlePlaybackError} />

            {playbackError !== undefined && playbackError !== '' && <p className="ui-error">{playbackError}</p>}
          </section>

          <aside className="watch-chat-panel">
            {chatAvailable ? (
              <Chat
                channelLogin={channelLogin}
                chatAvailable={chatAvailable}
                availableEmotes={availableEmotes}
                onStatusChange={handleChatStatusChange}
              />
            ) : (
              <div className="chat-offline">
                <p className="ui-muted">Connect Twitch to read and send messages.</p>
                <button type="button" className="ui-nav-chip" onClick={handleConnectTwitch}>
                  Connect Twitch
                </button>
              </div>
            )}
          </aside>
        </div>
      )}
    </section>
  );
}
