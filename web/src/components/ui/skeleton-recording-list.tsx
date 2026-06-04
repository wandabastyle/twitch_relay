import { Box, Stack } from '@mui/material';
import type { ReactElement } from 'react';
import { SkeletonText } from './skeleton-text';

interface SkeletonRecordingListProps {
  sections?: number;
  itemsPerSection?: number;
}

const DEFAULT_SECTIONS = 3;
const DEFAULT_ITEMS_PER_SECTION = 3;

export const SkeletonRecordingList = ({
  sections = DEFAULT_SECTIONS,
  itemsPerSection = DEFAULT_ITEMS_PER_SECTION,
}: SkeletonRecordingListProps): ReactElement => (
  <Stack spacing={4}>
    {Array.from({ length: sections }).map((_unused, sectionIndex) => (
      <Box key={sectionIndex}>
        <Box className="skeleton-section-header">
          <SkeletonText lines={1} width="120px" />
        </Box>
        <Box className="skeleton-list" sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {Array.from({ length: itemsPerSection }).map((_unusedItem, itemIndex) => (
            <Box
              key={`${sectionIndex}-${itemIndex}`}
              className="skeleton-item"
              sx={{
                alignItems: 'center',
                display: 'flex',
                justifyContent: 'space-between',
              }}
            >
              <Box className="skeleton-item-content" sx={{ display: 'flex', gap: 2 }}>
                <SkeletonText lines={1} width="60%" />
                <SkeletonText lines={1} width="40%" />
              </Box>
              <Box className="skeleton-item-actions" sx={{ display: 'flex', gap: 1 }}>
                <Box className="skeleton-action" />
                <Box className="skeleton-action" />
              </Box>
            </Box>
          ))}
        </Box>
      </Box>
    ))}
  </Stack>
);
