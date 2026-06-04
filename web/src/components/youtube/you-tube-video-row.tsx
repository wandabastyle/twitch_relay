import { Card, CardActionArea, CardContent, CardMedia, Typography, Box } from '@mui/material';
import type { ReactElement } from 'react';
import type { YoutubeVideo } from '../../api-client';
import { getYouTubeThumbnailUrl } from '../../api-client/youtube-progress';
import { formatDuration, formatViewCount } from '../../utils/youtube-format';

interface YouTubeVideoRowProps {
  video: YoutubeVideo;
  onClick: () => void;
}

export const YouTubeVideoRow = ({ video, onClick }: YouTubeVideoRowProps): ReactElement => (
  <Card className="youtube-video-row" sx={{ marginBottom: 1 }}>
    <CardActionArea
      onClick={onClick}
      sx={{ alignItems: 'flex-start', display: 'flex', padding: 2 }}
    >
      <Box sx={{ flexShrink: 0, marginRight: 2, position: 'relative' }}>
        <CardMedia
          alt={video.title}
          component="img"
          image={getYouTubeThumbnailUrl(video.video_id)}
          loading="lazy"
          sx={{ height: 90, objectFit: 'cover', width: 160 }}
        />
        <Box
          sx={{
            backgroundColor: 'rgba(0, 0, 0, 0.8)',
            borderRadius: 0.5,
            bottom: 4,
            color: 'white',
            fontSize: '0.75rem',
            padding: '2px 4px',
            position: 'absolute',
            right: 4,
          }}
        >
          {formatDuration(video.duration)}
        </Box>
      </Box>
      <CardContent sx={{ flex: 1, padding: 0 }}>
        <Typography
          className="youtube-video-title"
          gutterBottom
          sx={{
            WebkitBoxOrient: 'vertical',
            display: '-webkit-box',
            overflow: 'hidden',
          }}
          title={video.title}
          variant="subtitle1"
        >
          {video.title}
        </Typography>
        <Typography className="youtube-video-meta" color="text.secondary" variant="body2">
          {video.author} · {formatViewCount(video.view_count)} views · {video.published_text}
        </Typography>
        {video.description !== undefined && video.description !== '' && (
          <Typography
            className="youtube-video-description"
            color="text.secondary"
            sx={{
              WebkitBoxOrient: 'vertical',
              display: '-webkit-box',
              marginTop: 1,
              overflow: 'hidden',
            }}
            title={video.description}
            variant="caption"
          >
            {video.description}
          </Typography>
        )}
      </CardContent>
    </CardActionArea>
  </Card>
);
