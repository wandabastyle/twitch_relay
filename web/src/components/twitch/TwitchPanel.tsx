import type { ReactElement, ReactNode } from 'react';

interface TwitchPanelProps {
  children: ReactNode;
}

export function TwitchPanel({ children }: TwitchPanelProps): ReactElement {
  return <section className="twitch-panel">{children}</section>;
}
