import { Box, Stack } from '@mui/material';
import type { ReactElement } from 'react';
import { SkeletonText } from './skeleton-text';
import { SkeletonThumbnail } from './skeleton-thumbnail';

interface SkeletonVideoListProps {
  count?: number;
}

const DEFAULT_COUNT = 5;

export const SkeletonVideoList = ({
  count = DEFAULT_COUNT,
}: SkeletonVideoListProps): ReactElement => (
  <Stack spacing={2}>
    {Array.from({ length: count }).map((_unused, index) => (
      <Box key={index} sx={{ display: 'flex', gap: 2 }}>
        <SkeletonThumbnail width="240px" aspectRatio="16 / 9" />
        <SkeletonText lines={2} width="85%" />
      </Box>
    ))}
  </Stack>
);
