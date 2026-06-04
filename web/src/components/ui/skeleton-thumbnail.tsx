import { Skeleton } from '@mui/material';
import type { ReactElement } from 'react';

interface SkeletonThumbnailProps {
  aspectRatio?: string;
  borderRadius?: string;
  height?: string;
  width?: string;
}

export const SkeletonThumbnail = ({
  aspectRatio = '16 / 9',
  borderRadius = '0.5rem',
  height = 'auto',
  width = '100%',
}: SkeletonThumbnailProps): ReactElement => (
  <Skeleton
    variant="rectangular"
    sx={{
      aspectRatio,
      borderRadius,
      height,
      width,
    }}
  />
);
