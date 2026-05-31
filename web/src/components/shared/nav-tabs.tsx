import type { ReactElement } from 'react';

// YouTube-specific tabs
interface YouTubeNavTabsProps {
  activeTab: 'subscriptions' | 'recent' | 'playlists';
}

// YouTube-specific tabs
interface YouTubeNavTabsProps {
  activeTab: 'subscriptions' | 'recent' | 'playlists';
}

export const YouTubeNavTabs = ({ activeTab }: YouTubeNavTabsProps): ReactElement => {
  const items = [
    { href: '/youtube', label: 'Subscriptions' },
    { href: '/youtube/recent', label: 'Recent' },
    { href: '/youtube/playlists', label: 'Playlists' },
  ];

  return (
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
}
