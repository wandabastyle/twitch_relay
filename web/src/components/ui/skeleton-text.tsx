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
}: SkeletonTextProps): ReactElement => {
  const style: React.CSSProperties & Record<string, string | number> = {
    '--lines': lines,
    '--width': width,
  };
  return (
    <div
      className="skeleton-text"
      style={style}
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
};
