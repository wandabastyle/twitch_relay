import { Fade } from '@mui/material';
import type { ReactElement } from 'react';

interface LoadedFadeProps {
  loaded?: boolean;
  duration?: number;
  children: ReactElement;
}

const DEFAULT_DURATION_MS = 280;

export const LoadedFade = ({
  loaded = true,
  duration = DEFAULT_DURATION_MS,
  children,
}: LoadedFadeProps): ReactElement => (
  <Fade in={loaded} timeout={duration}>
    <>{children}</>
  </Fade>
);
