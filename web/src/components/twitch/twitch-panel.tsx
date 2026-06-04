import { Paper } from '@mui/material';
import type { ReactElement, ReactNode } from 'react';

interface TwitchPanelProps {
  children: ReactNode;
}

export const TwitchPanel = ({ children }: TwitchPanelProps): ReactElement => (
  <Paper component="section" elevation={1} sx={{ padding: 2 }}>
    {children}
  </Paper>
);
