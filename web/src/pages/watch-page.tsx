import { type ReactElement, useCallback, useEffect, useRef, useState } from 'react';
import {
  getChatEmotes,
  getLiveStatus,
  getRecordings,
  getTwitchConnectUrl,
  getTwitchStatus,
  getWatchSession,
  startRecording,
  stopRecording,
  type ActiveRecording,
  type EmoteItem,
  type WatchSessionResponse,
} from '../api-client';
import type { ChatComposerHandle } from '../components/watch/chat-composer';
import { RecordingButton } from '../components/watch/recording-button';
import type { VideoControlsHandle } from '../components/watch/use-video-controls';
import { WatchContent } from '../components/watch/watch-content';
import { WatchPageMeta } from '../components/watch/watch-page-meta';
import { useChatOnlyMode } from '../hooks/use-chat-only-mode';
import { useKeyboardShortcuts, useToggleCallback } from '../hooks/use-keyboard-shortcuts';
import { useRouter } from '../hooks/use-router';
import { useWatchPreferences } from '../hooks/use-watch-preferences';

const EMPTY_MESSAGE_LENGTH = 0;
const ERROR_MISSING_TICKET = 'Missing watch ticket.';
const ERROR_SESSION_FAILED = 'Failed to initialize watch session.';
const RELAY_PARAM = '1';
const LIVE_STATUS_POLL_MS = 30_000;

interface ChatStatus {
  available: boolean;
  connected: boolean;
  message: string;
}

interface WatchMetadata {
  activeRecording: ActiveRecording | undefined;
  channelLogin: string;
  displayName?: string;
  game?: string;
  handleStartRecording: () => Promise<void>;
  handleStopRecording: () => Promise<void>;
  live: boolean;
  manifestUrl: string;
  profileUrl?: string;
  title?: string;
  viewerCount?: number;
  watchError?: string;
  watchLoading: boolean;
}

interface WatchPageProps {
  minimized?: boolean;
  ticketOverride?: string;
}

const readMessage = (err: unknown, fallback: string): string => {
  if (err instanceof Error && err.message.trim().length > EMPTY_MESSAGE_LENGTH) {
    return err.message;
  }
  return fallback;
};

const isRelayForced = (): boolean => {
  if (typeof globalThis === 'undefined') {
    return false;
  }
  return new URLSearchParams(globalThis.location.search).get('relay') === RELAY_PARAM;
};

const applySessionToState = (
  session: WatchSessionResponse,
  setters: {
    setChannelLogin: (value: string) => void;
    setDisplayName: (value?: string) => void;
    setProfileUrl: (value?: string) => void;
    setTitle: (value?: string) => void;
    setGame: (value?: string) => void;
    setViewerCount: (value?: number) => void;
    setLive: (value: boolean) => void;
    setManifestUrl: (value: string) => void;
  },
): void => {
  setters.setChannelLogin(session.channel);
  setters.setDisplayName(session.display_name);
  setters.setProfileUrl(session.profile_url);
  setters.setTitle(session.title);
  setters.setGame(session.game);
  setters.setViewerCount(session.viewer_count);
  setters.setLive(session.live);
  setters.setManifestUrl(session.manifest_url);
};

const useWatchMetadata = (ticket: string): WatchMetadata => {
  const [channelLogin, setChannelLogin] = useState('');
  const [displayName, setDisplayName] = useState<string | undefined>();
  const [profileUrl, setProfileUrl] = useState<string | undefined>();
  const [title, setTitle] = useState<string | undefined>();
  const [game, setGame] = useState<string | undefined>();
  const [viewerCount, setViewerCount] = useState<number | undefined>();
  const [live, setLive] = useState(false);
  const [manifestUrl, setManifestUrl] = useState('');
  const [watchLoading, setWatchLoading] = useState(true);
  const [watchError, setWatchError] = useState<string | undefined>();
  const [activeRecording, setActiveRecording] = useState<ActiveRecording | undefined>();

  const channelLoginRef = useRef(channelLogin);
  channelLoginRef.current = channelLogin;

  const initialize = useCallback(async (): Promise<string> => {
    if (ticket === '') {
      setWatchError(ERROR_MISSING_TICKET);
      setWatchLoading(false);
      return '';
    }

    setWatchLoading(true);
    setWatchError(undefined);

    try {
      const session = await getWatchSession(ticket, isRelayForced());
      applySessionToState(session, {
        setChannelLogin,
        setDisplayName,
        setGame,
        setLive,
        setManifestUrl,
        setProfileUrl,
        setTitle,
        setViewerCount,
      });
      setWatchLoading(false);
      return session.channel;
    } catch (error) {
      setWatchError(readMessage(error, ERROR_SESSION_FAILED));
      setWatchLoading(false);
      return '';
    }
  }, [ticket]);

  const refreshLiveStatus = useCallback(async (): Promise<void> => {
    const currentChannel = channelLoginRef.current;
    if (currentChannel === '') {
      return;
    }

    try {
      const status = await getLiveStatus();
      const channelStatus = status.channels[currentChannel];
      if (!('live' in channelStatus)) {
        return;
      }

      setLive(channelStatus.live);
      setTitle(channelStatus.title);
      setGame(channelStatus.game);
      setViewerCount(channelStatus.viewer_count);
      setDisplayName(channelStatus.display_name);
      setProfileUrl(channelStatus.profile_url);
    } catch {
      // Keep existing metadata on refresh failure.
    }
  }, []);

  const findActiveRecording = useCallback(
    (recordings: { active: readonly ActiveRecording[] }): ActiveRecording | undefined =>
      recordings.active.find((recording) => recording.channel_login === channelLoginRef.current),
    [],
  );

  const refreshRecordings = useCallback(async (): Promise<void> => {
    const currentChannel = channelLoginRef.current;
    if (currentChannel === '') {
      return;
    }

    try {
      const recordings = await getRecordings();
      setActiveRecording(findActiveRecording(recordings));
    } catch {
      // Keep existing recording state on refresh failure.
    }
  }, [findActiveRecording]);

  const handleStartRecording = useCallback(async (): Promise<void> => {
    await startRecording(channelLoginRef.current, undefined, title);
    await refreshRecordings();
  }, [title, refreshRecordings]);

  const handleStopRecording = useCallback(async (): Promise<void> => {
    await stopRecording(channelLoginRef.current);
    await refreshRecordings();
  }, [refreshRecordings]);

  useEffect(() => {
    void initialize();
  }, [ticket, initialize]);

  useEffect((): (() => void) | undefined => {
    if (channelLogin === '') {
      return undefined;
    }

    void refreshRecordings();

    const interval = setInterval(() => {
      void refreshLiveStatus();
      void refreshRecordings();
    }, LIVE_STATUS_POLL_MS);

    return (): void => {
      clearInterval(interval);
    };
  }, [channelLogin, refreshLiveStatus, refreshRecordings]);

  return {
    activeRecording,
    channelLogin,
    displayName,
    game,
    handleStartRecording,
    handleStopRecording,
    live,
    manifestUrl,
    profileUrl,
    title,
    viewerCount,
    watchError,
    watchLoading,
  };
};

const useChatSetup = (
  channelLogin: string,
): {
  availableEmotes: EmoteItem[];
  chatAvailable: boolean;
  handleChatStatusChange: (status: ChatStatus) => void;
  twitchStatusChecked: boolean;
} => {
  const [chatAvailable, setChatAvailable] = useState(false);
  const [availableEmotes, setAvailableEmotes] = useState<EmoteItem[]>([]);
  const [twitchStatusChecked, setTwitchStatusChecked] = useState(false);

  const loadEmotes = useCallback(async (channel: string): Promise<void> => {
    if (channel === '') {
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

  useEffect(() => {
    if (channelLogin === '') {
      return;
    }

    void setupChat(channelLogin);
  }, [channelLogin, setupChat]);

  const handleChatStatusChange = useCallback((status: ChatStatus): void => {
    setChatAvailable(status.available);
  }, []);

  return {
    availableEmotes,
    chatAvailable,
    handleChatStatusChange,
    twitchStatusChecked,
  };
};

interface TwitchUser {
  connected: boolean;
  display_name?: string;
  login?: string;
}

const useCurrentTwitchUser = (): TwitchUser => {
  const [user, setUser] = useState<TwitchUser>({ connected: false });

  useEffect(() => {
    void (async (): Promise<void> => {
      try {
        const status = await getTwitchStatus();
        setUser({
          connected: status.connected,
          display_name: status.display_name,
          login: status.login,
        });
      } catch {
        setUser({ connected: false });
      }
    })();
  }, []);

  return user;
};

export const WatchPage = ({
  minimized = false,
  ticketOverride,
}: WatchPageProps = {}): ReactElement => {
  const { navigate, page } = useRouter();
  const ticket = ticketOverride ?? page.params.ticket;

  const {
    activeRecording,
    channelLogin,
    displayName,
    game,
    handleStartRecording,
    handleStopRecording,
    live,
    manifestUrl,
    profileUrl,
    title,
    viewerCount,
    watchError,
    watchLoading,
  } = useWatchMetadata(ticket);

  const { availableEmotes, chatAvailable, handleChatStatusChange, twitchStatusChecked } =
    useChatSetup(channelLogin);

  const { isChatCollapsed, setIsChatCollapsed, setTheaterMode, theaterMode } =
    useWatchPreferences();
  const currentTwitchUser = useCurrentTwitchUser();
  const { chatOnly, toggleChatCollapse, toggleChatOnly } = useChatOnlyMode(
    ticket,
    setIsChatCollapsed,
  );
  const [playbackError, setPlaybackError] = useState<string | undefined>();
  const composerRef = useRef<ChatComposerHandle | null>(null);
  const videoPlayerRef = useRef<VideoControlsHandle | null>(null);

  const toggleTheaterMode = useToggleCallback(setTheaterMode);

  const handlePlaybackError = useCallback((msg: string): void => {
    setPlaybackError(msg);
  }, []);

  const handleFocusChat = useCallback((): void => {
    composerRef.current?.focus();
  }, []);

  const handleToggleFullscreen = useCallback((): void => {
    videoPlayerRef.current?.enterFullscreen();
  }, []);

  const handleToggleMute = useCallback((): void => {
    videoPlayerRef.current?.toggleMute();
  }, []);

  useKeyboardShortcuts({
    enabled: !minimized,
    onFocusChat: handleFocusChat,
    onFullscreen: handleToggleFullscreen,
    onMute: handleToggleMute,
    onTheater: toggleTheaterMode,
    theaterMode,
  });

  useEffect(() => {
    if (minimized && theaterMode) {
      setTheaterMode(false);
    }
  }, [minimized, setTheaterMode, theaterMode]);

  const handleBackToChannels = useCallback((): void => {
    navigate('/twitch');
  }, [navigate]);

  const connectTwitchUrl = getTwitchConnectUrl();
  const handleConnectTwitch = useCallback((): void => {
    globalThis.location.href = connectTwitchUrl;
  }, [connectTwitchUrl]);

  return (
    <section
      className={`watch-page${minimized ? ' watch-page--minimized' : ''}`}
      data-theater={!minimized && theaterMode ? 'true' : 'false'}
    >
      <header className="watch-page-header">
        <WatchPageMeta
          channelLogin={channelLogin}
          displayName={displayName}
          game={game}
          live={live}
          profileUrl={profileUrl}
          title={title}
          viewerCount={viewerCount}
        />
        <div className="watch-page-actions">
          {channelLogin !== '' && (
            <RecordingButton
              channelLogin={channelLogin}
              recording={activeRecording}
              onStart={handleStartRecording}
              onStop={handleStopRecording}
            />
          )}
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

      <WatchContent
        availableEmotes={availableEmotes}
        channelLogin={channelLogin}
        chatAvailable={chatAvailable}
        chatOnly={chatOnly}
        currentUserDisplayName={currentTwitchUser.display_name}
        currentUserLogin={currentTwitchUser.login}
        handleChatStatusChange={handleChatStatusChange}
        handleConnectTwitch={handleConnectTwitch}
        handlePlaybackError={handlePlaybackError}
        handleToggleCollapse={toggleChatCollapse}
        handleToggleChatOnly={toggleChatOnly}
        isChatCollapsed={isChatCollapsed}
        manifestUrl={manifestUrl}
        onToggleTheater={toggleTheaterMode}
        playbackError={playbackError}
        theaterMode={!minimized && theaterMode}
        videoPlayerRef={videoPlayerRef}
        watchError={watchError}
        watchLoading={watchLoading}
      />
    </section>
  );
};
