import type { ReactElement } from 'react';
import { SkeletonText } from './SkeletonText';
import { SkeletonThumbnail } from './SkeletonThumbnail';

interface SkeletonVideoListProps {
  count?: number;
}

const DEFAULT_COUNT = 5;

export const SkeletonVideoList = ({ count = DEFAULT_COUNT }: SkeletonVideoListProps): ReactElement => {
  return (
    <div className="skeleton-video-list">
      {Array.from({ length: count }).map((_, index) => (
        <div key={index} className="skeleton-video-row">
          <SkeletonThumbnail width="240px" aspectRatio="16 / 9" />
          <div className="skeleton-video-info">
            <SkeletonText lines={2} width="85%" />
          </div>
        </div>
      ))}
    </div>
  );
}
