import { Tabs, Tab } from '@mui/material';
import type { ReactElement } from 'react';

interface YouTubeNavProps {
  activeTab: 'subscriptions' | 'recent' | 'playlists';
}

export const YouTubeNav = ({ activeTab }: YouTubeNavProps): ReactElement => (
  <Tabs value={activeTab} centered>
    <Tab label="Subscriptions" value="subscriptions" component="a" href="/youtube" />
    <Tab label="Recent" value="recent" component="a" href="/youtube/recent" />
    <Tab label="Playlists" value="playlists" component="a" href="/youtube/playlists" />
  </Tabs>
);
