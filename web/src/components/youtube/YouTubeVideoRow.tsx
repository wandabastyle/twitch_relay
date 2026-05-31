import type { ReactElement } from 'react';
import type { YoutubeVideo } from '../../lib/api-client';
import { getYouTubeThumbnailUrl } from '../../lib/api-client/youtube-progress';
import { formatDuration, formatViewCount } from '../../lib/youtube/format';

interface YouTubeVideoRowProps {
  video: YoutubeVideo;
  onClick: () => void;
}

export function YouTubeVideoRow({ video, onClick }: YouTubeVideoRowProps): ReactElement {
  return (
    <button type="button" className="youtube-video-row" onClick={onClick}>
      <div className="youtube-video-thumb-wrap">
        <img
          className="youtube-video-thumb"
          src={getYouTubeThumbnailUrl(video.video_id)}
          alt={video.title}
          loading="lazy"
        />
        <span className="youtube-video-duration">{formatDuration(video.duration)}</span>
      </div>
      <div className="youtube-video-info">
        <h3 className="youtube-video-title" title={video.title}>
          {video.title}
        </h3>
        <div className="youtube-video-meta">
          {video.author} · {formatViewCount(video.view_count)} views · {video.published_text}
        </div>
        {video.description && (
          <p className="youtube-video-description" title={video.description}>
            {video.description}
          </p>
        )}
      </div>
    </button>
  );
}
