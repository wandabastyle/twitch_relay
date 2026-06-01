import type { ReactElement } from 'react';

interface IncompleteSectionHeaderProps {
  readonly incompleteCount: number;
  readonly recordingsChannelFilter: string;
  readonly shownIncompleteLength: number;
  readonly selectedCount: number;
  readonly mergingRecordingKey: string | undefined;
  readonly onRequestProcessIncompleteFiles: (channelLogin: string) => void;
}

const EMPTY_LENGTH = 0;
const MINIMUM_SELECTION = 1;
const SINGLE_SELECTION = 1;

export const IncompleteSectionHeader = ({
  incompleteCount,
  recordingsChannelFilter,
  shownIncompleteLength,
  selectedCount,
  mergingRecordingKey,
  onRequestProcessIncompleteFiles,
}: IncompleteSectionHeaderProps): ReactElement => {
  const isMerging = mergingRecordingKey === recordingsChannelFilter;

  return (
    <div className="incomplete-section-header">
      <h2>Incomplete ({incompleteCount})</h2>
      {recordingsChannelFilter !== 'all' && shownIncompleteLength > EMPTY_LENGTH && (
        <button
          type="button"
          className="merge-btn"
          onClick={() => {
            onRequestProcessIncompleteFiles(recordingsChannelFilter);
          }}
          disabled={selectedCount < MINIMUM_SELECTION || isMerging}
        >
          {isMerging ? (
            <>
              <span className="merge-btn-spinner" />
              {selectedCount === SINGLE_SELECTION ? 'Finalizing...' : 'Merging...'}
            </>
          ) : (
            <>
              {selectedCount === SINGLE_SELECTION
                ? 'Finalize selected'
                : `Merge selected (${selectedCount})`}
            </>
          )}
        </button>
      )}
    </div>
  );
};
