import type { ReactElement } from 'react';
import TwitchLayout from './components/layout/twitch-layout';
import YouTubeLayout from './components/layout/you-tube-layout';
import { useRouter } from './hooks/use-router';
import {
  IndexRedirect,
  TwitchHomePage,
  TwitchChannelPage,
  TwitchRecordingsPage,
  TwitchRecordingPlayerPage,
  WatchPage,
  QrLoginPage,
  YouTubeHomePage,
  YouTubeRecentPage,
  YouTubePlaylistsPage,
  YouTubeChannelPage,
  YouTubePlaylistPage,
  YouTubeWatchPage,
} from './pages';

const NotFoundPage = (): ReactElement => (
  <TwitchLayout>
    <div className="ui-page-panel">
      <h1>404 - Not Found</h1>
      <p className="ui-muted">The page you are looking for does not exist.</p>
    </div>
  </TwitchLayout>
);

const MIN_LENGTH = 1;

/**
 * Render route based on current path.
 * Uses if-else chain to avoid switch-exhaustiveness lint error.
 */
const renderRoute = (path: string, params: Record<string, string>): ReactElement => {
  // Index redirect
  if (path === '/') {
    return <IndexRedirect />;
  }

  // Twitch routes
  if (path === '/twitch') {
    return (
      <TwitchLayout>
        <TwitchHomePage />
      </TwitchLayout>
    );
  }

  if (path.startsWith('/twitch/channels/')) {
    const { login } = params;
    if (login !== undefined && login.length >= MIN_LENGTH) {
      return (
        <TwitchLayout>
          <TwitchChannelPage />
        </TwitchLayout>
      );
    }
    return <NotFoundPage />;
  }

  if (path === '/twitch/recordings') {
    return (
      <TwitchLayout>
        <TwitchRecordingsPage />
      </TwitchLayout>
    );
  }

  if (path === '/twitch/recordings/play') {
    return (
      <TwitchLayout>
        <TwitchRecordingPlayerPage />
      </TwitchLayout>
    );
  }

  // Watch route (no layout wrapper)
  if (path.startsWith('/watch/')) {
    const { ticket } = params;
    if (ticket !== undefined && ticket.length >= MIN_LENGTH) {
      return <WatchPage />;
    }
    return <NotFoundPage />;
  }

  // QR Login route (no layout wrapper)
  if (path.startsWith('/qr-login/')) {
    const { token } = params;
    if (token !== undefined && token.length >= MIN_LENGTH) {
      return <QrLoginPage token={token} />;
    }
    return <NotFoundPage />;
  }

  // YouTube routes
  if (path === '/youtube') {
    return (
      <YouTubeLayout>
        <YouTubeHomePage />
      </YouTubeLayout>
    );
  }

  if (path === '/youtube/recent') {
    return (
      <YouTubeLayout>
        <YouTubeRecentPage />
      </YouTubeLayout>
    );
  }

  if (path === '/youtube/playlists') {
    return (
      <YouTubeLayout>
        <YouTubePlaylistsPage />
      </YouTubeLayout>
    );
  }

  if (path.startsWith('/youtube/channel/')) {
    const channelId = params.channel_id;
    if (channelId !== undefined && channelId.length >= MIN_LENGTH) {
      return (
        <YouTubeLayout>
          <YouTubeChannelPage channel_id={channelId} />
        </YouTubeLayout>
      );
    }
    return <NotFoundPage />;
  }

  if (path.startsWith('/youtube/playlist/')) {
    const playlistId = params.playlist_id;
    if (playlistId !== undefined && playlistId.length >= MIN_LENGTH) {
      return (
        <YouTubeLayout>
          <YouTubePlaylistPage playlist_id={playlistId} />
        </YouTubeLayout>
      );
    }
    return <NotFoundPage />;
  }

  if (path.startsWith('/youtube/watch/')) {
    const videoId = params.video_id;
    if (videoId !== undefined && videoId.length >= MIN_LENGTH) {
      return (
        <YouTubeLayout>
          <YouTubeWatchPage video_id={videoId} />
        </YouTubeLayout>
      );
    }
    return <NotFoundPage />;
  }

  // Default: 404
  return <NotFoundPage />;
}

/**
 * Main application component with routing.
 */
export default function App(): ReactElement {
  const { page } = useRouter();

  return renderRoute(page.path, page.params);
}
