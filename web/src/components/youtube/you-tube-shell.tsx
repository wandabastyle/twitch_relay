import type { ReactElement, ReactNode } from 'react';
import { navigate } from '../../router';
import { RelayHeader } from '../shared/relay-header';
import { YouTubeNav } from './you-tube-nav';

interface YouTubeShellProps {
  activeTab: 'subscriptions' | 'recent' | 'playlists';
  subtitle?: string;
  children: ReactNode;
}

export const YouTubeShell = ({
  activeTab,
  subtitle = 'Invidious subscriptions',
  children,
}: YouTubeShellProps): ReactElement => {
  const switchToTwitch = (): void => {
    navigate('/twitch');
  };

  return (
    <section className="youtube-panel">
      <RelayHeader
        eyebrow="Private Deck"
        title="YouTube Relay"
        subtitleText={subtitle}
        onToggle={switchToTwitch}
        toggleLabel="Switch to Twitch Relay"
      />
      <YouTubeNav activeTab={activeTab} />
      {children}
    </section>
  );
}
