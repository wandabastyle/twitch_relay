import { Box, Paper, Typography } from '@mui/material';
import { FolderOpen, ListVideo, Tv, Users, Video } from 'lucide-react';
import type { ReactElement } from 'react';

interface EmptyStateProps {
  action?: ReactElement;
  description?: string;
  title: string;
  variant?: 'default' | 'channels' | 'videos' | 'playlists' | 'recordings';
}

const icons = {
  channels: Users,
  default: FolderOpen,
  playlists: ListVideo,
  recordings: Tv,
  videos: Video,
};

export const EmptyState = ({
  action,
  description,
  title,
  variant = 'default',
}: EmptyStateProps): ReactElement => {
  const Icon = icons[variant];

  return (
    <Paper className="empty-state" elevation={0} role="status">
      <Box
        sx={{
          alignItems: 'center',
          display: 'flex',
          flexDirection: 'column',
          py: 4,
          textAlign: 'center',
        }}
      >
        <Box className="empty-icon" sx={{ mb: 2 }}>
          <Icon size={32} strokeWidth={1.5} />
        </Box>
        <Typography className="empty-title" component="h3" gutterBottom variant="h6">
          {title}
        </Typography>
        {description !== undefined && description !== '' && (
          <Typography className="empty-description" sx={{ mb: 1 }} variant="body2">
            {description}
          </Typography>
        )}
        {action !== undefined && (
          <Box className="empty-action" sx={{ mt: 2 }}>
            {action}
          </Box>
        )}
      </Box>
    </Paper>
  );
};
