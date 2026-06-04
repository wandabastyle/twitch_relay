import { Box, Stack } from '@mui/material';
import type { ReactElement } from 'react';
import { SkeletonText } from './skeleton-text';
import { SkeletonThumbnail } from './skeleton-thumbnail';

interface SkeletonMediaListProps {
  count?: number;
  avatarSize?: string;
}

const DEFAULT_COUNT = 8;
const DEFAULT_AVATAR_SIZE = '74px';

export const SkeletonMediaList = ({
  count = DEFAULT_COUNT,
  avatarSize = DEFAULT_AVATAR_SIZE,
}: SkeletonMediaListProps): ReactElement => (
  <Stack spacing={2}>
    {Array.from({ length: count }).map((_unused, index) => (
      <Box key={index} sx={{ alignItems: 'center', display: 'flex', gap: 2 }}>
        <SkeletonThumbnail
          width={avatarSize}
          height={avatarSize}
          aspectRatio="1 / 1"
          borderRadius="50%"
        />
        <SkeletonText lines={1} width="60%" />
      </Box>
    ))}
  </Stack>
);
