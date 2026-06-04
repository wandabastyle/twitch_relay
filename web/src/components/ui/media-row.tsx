import { Avatar } from '@mui/material';
import Card from '@mui/material/Card';
import CardActionArea from '@mui/material/CardActionArea';
import Typography from '@mui/material/Typography';
import type { ReactElement, ReactNode } from 'react';

interface MediaRowProps {
  extraClass?: string;
  meta?: ReactNode;
  onClick: () => void;
  title: string;
  visual: ReactNode;
}

export const MediaRow = ({
  extraClass = '',
  meta,
  onClick,
  title,
  visual,
}: MediaRowProps): ReactElement => (
  <Card className={`ui-card ui-card-interactive ui-media-row ${extraClass}`} elevation={0}>
    <CardActionArea
      onClick={onClick}
      component="button"
      sx={{ alignItems: 'center', display: 'flex', padding: 0, textAlign: 'left' }}
    >
      <div className="ui-media-visual" style={{ marginRight: 16 }}>
        {visual}
      </div>
      <div className="ui-media-main" style={{ flex: 1 }}>
        <Typography
          className="ui-media-title"
          component="span"
          title={title}
          variant="subtitle1"
          noWrap
        >
          {title}
        </Typography>
        {meta}
      </div>
    </CardActionArea>
  </Card>
);

interface MediaRowAvatarProps {
  alt?: string;
  fallbackInitial: string;
  size?: string;
  src?: string | undefined;
}

export const MediaRowAvatar = ({
  alt,
  fallbackInitial,
  size = '74px',
  src,
}: MediaRowAvatarProps): ReactElement => {
  const sx = { height: size, width: size };
  if (src === undefined || src === '') {
    return (
      <Avatar
        className="ui-avatar ui-avatar-fallback"
        sx={sx}
        alt={alt ?? fallbackInitial}
        aria-label={alt ?? fallbackInitial}
      >
        {fallbackInitial}
      </Avatar>
    );
  }

  return <Avatar className="ui-avatar" src={src} alt={alt ?? fallbackInitial} sx={sx} />;
};

interface MediaRowMetaProps {
  children: ReactNode;
}

export const MediaRowMeta = ({ children }: MediaRowMetaProps): ReactElement => (
  <Typography component="span" className="ui-media-meta" variant="body2">
    {children}
  </Typography>
);
