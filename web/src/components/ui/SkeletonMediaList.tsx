import type { ReactElement } from 'react';
import { SkeletonText } from './SkeletonText';
import { SkeletonThumbnail } from './SkeletonThumbnail';

interface SkeletonMediaListProps {
  count?: number;
  avatarSize?: string;
}

const DEFAULT_COUNT = 8;
const DEFAULT_AVATAR_SIZE = '74px';

export const SkeletonMediaList = ({
  count = DEFAULT_COUNT,
  avatarSize = DEFAULT_AVATAR_SIZE,
}: SkeletonMediaListProps): ReactElement => {
  return (
    <div className="skeleton-media-list">
      {Array.from({ length: count }).map((_, index) => (
        <div key={index} className="skeleton-media-row">
          <SkeletonThumbnail
            width={avatarSize}
            height={avatarSize}
            aspectRatio="1 / 1"
            borderRadius="50%"
          />
          <div className="skeleton-media-info">
            <SkeletonText lines={1} width="60%" />
          </div>
        </div>
      ))}
    </div>
  );
}
