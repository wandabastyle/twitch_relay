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
  <div className="skeleton-recordings">
    {Array.from({ length: sections }).map((_unused, sectionIndex) => (
      <div key={sectionIndex} className="skeleton-section">
        <div className="skeleton-section-header">
          <SkeletonText lines={1} width="120px" />
        </div>
        <div className="skeleton-list">
          {Array.from({ length: itemsPerSection }).map((_unusedItem, itemIndex) => (
            <div key={`${sectionIndex}-${itemIndex}`} className="skeleton-item">
              <div className="skeleton-item-content">
                <SkeletonText lines={1} width="60%" />
                <SkeletonText lines={1} width="40%" />
              </div>
              <div className="skeleton-item-actions">
                <div className="skeleton-action" />
                <div className="skeleton-action" />
              </div>
            </div>
          ))}
        </div>
      </div>
    ))}
  </div>
);
