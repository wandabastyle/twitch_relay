import { Tabs, Tab } from '@mui/material';
import type { ReactElement } from 'react';

interface YouTubeNavTabsProps {
  activeTab: 'subscriptions' | 'recent' | 'playlists';
}

export const YouTubeNavTabs = ({ activeTab }: YouTubeNavTabsProps): ReactElement => (
  <Tabs value={activeTab} centered>
    <Tab label="Subscriptions" value="subscriptions" component="a" href="/youtube" />
    <Tab label="Recent" value="recent" component="a" href="/youtube/recent" />
    <Tab label="Playlists" value="playlists" component="a" href="/youtube/playlists" />
  </Tabs>
);
