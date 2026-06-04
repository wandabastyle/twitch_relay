import { Skeleton } from '@mui/material';
import type { ReactElement } from 'react';

interface SkeletonTextProps {
  lines?: number;
  width?: string;
}

const DEFAULT_LINES = 1;
const DEFAULT_WIDTH = '100%';
const LAST_LINE_OFFSET = 1;

export const SkeletonText = ({
  lines = DEFAULT_LINES,
  width = DEFAULT_WIDTH,
}: SkeletonTextProps): ReactElement => {
  const lastIndex = lines - LAST_LINE_OFFSET;
  return (
    <>
      {Array.from({ length: lines }).map((_unused, index) => (
        <Skeleton key={index} variant="text" width={index === lastIndex ? width : '100%'} />
      ))}
    </>
  );
};
