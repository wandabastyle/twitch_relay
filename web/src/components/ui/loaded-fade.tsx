import type { ReactElement, ReactNode } from 'react';

interface LoadedFadeProps {
  loaded?: boolean;
  duration?: number;
  children: ReactNode;
}

const DEFAULT_DURATION_MS = 280;

export const LoadedFade = ({
  loaded = true,
  duration = DEFAULT_DURATION_MS,
  children,
}: LoadedFadeProps): ReactElement => (
  <div
    className={`loaded-fade ${loaded ? 'loaded' : ''}`}
    style={{ '--loaded-fade-duration': `${duration}ms` } as React.CSSProperties}
  >
    {children}
  </div>
);
