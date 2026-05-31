import type { ReactElement } from 'react';

interface SkeletonTextProps {
  lines?: number;
  width?: string;
}

const DEFAULT_LINES = 1;
const DEFAULT_WIDTH = '100%';
const LAST_LINE_INDEX = 1;

export const SkeletonText = ({
  lines = DEFAULT_LINES,
  width = DEFAULT_WIDTH,
}: SkeletonTextProps): ReactElement => (
  <div
    className="skeleton-text"
    style={{ '--lines': lines, '--width': width } as React.CSSProperties}
  >
    {Array.from({ length: lines }).map((_unused, index) => (
      <div
        key={index}
        className="skeleton-line"
        style={{ width: index === lines - LAST_LINE_INDEX ? width : '100%' }}
      />
    ))}
  </div>
);
