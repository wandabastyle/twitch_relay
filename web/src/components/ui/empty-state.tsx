import { FolderOpen, ListVideo, Tv, Users, Video } from 'lucide-react';
import type { ReactElement, ReactNode } from 'react';

interface EmptyStateProps {
  title: string;
  description?: string;
  action?: ReactNode;
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
  title,
  description,
  action,
  variant = 'default',
}: EmptyStateProps): ReactElement => {
  const Icon = icons[variant];

  return (
    <div className="empty-state" role="status">
      <div className="empty-icon">
        <Icon size={32} strokeWidth={1.5} />
      </div>
      <h3 className="empty-title">{title}</h3>
      {description !== undefined && description !== '' && (
        <p className="empty-description">{description}</p>
      )}
      {action !== undefined && action !== null && <div className="empty-action">{action}</div>}
    </div>
  );
};
