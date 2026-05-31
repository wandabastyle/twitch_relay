import type { ReactElement, ReactNode } from 'react';

interface TwitchPanelProps {
  children: ReactNode;
}

export const TwitchPanel = ({ children }: TwitchPanelProps): ReactElement => (
  <section className="twitch-panel">{children}</section>
);
