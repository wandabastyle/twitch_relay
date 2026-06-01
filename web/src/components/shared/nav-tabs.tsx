import type { ReactElement } from 'react';

interface YouTubeNavTabsProps {
  activeTab: 'subscriptions' | 'recent' | 'playlists';
}

export const YouTubeNavTabs = ({ activeTab }: YouTubeNavTabsProps): ReactElement => (
  <nav className="youtube-nav">
    <a href="/youtube" className={`nav-link ${activeTab === 'subscriptions' ? 'active' : ''}`}>
      Subscriptions
    </a>
    <a href="/youtube/recent" className={`nav-link ${activeTab === 'recent' ? 'active' : ''}`}>
      Recent
    </a>
    <a
      href="/youtube/playlists"
      className={`nav-link ${activeTab === 'playlists' ? 'active' : ''}`}
    >
      Playlists
    </a>
  </nav>
);
