import type { ReactElement } from 'react';

interface SkeletonTextProps {
  lines?: number;
  width?: string;
}

const DEFAULT_LINES = 1;
const DEFAULT_WIDTH = '100%';

export const SkeletonText = ({
  lines = DEFAULT_LINES,
  width = DEFAULT_WIDTH,
}: SkeletonTextProps): ReactElement => (
  <div
    className="skeleton-text"
    style={{ '--lines': lines, '--width': width } as React.CSSProperties}
  >
    {Array.from({ length: lines }).map((_, i) => (
      <div
        key={i}
        className="skeleton-line"
        style={{ width: i === lines - 1 ? width : '100%' }}
      />
    ))}
  </div>
);
