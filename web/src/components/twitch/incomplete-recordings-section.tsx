import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';
import { IncompleteRecordingRow } from './incomplete-recording-row';
import { IncompleteSectionHeader } from './incomplete-section-header';

interface IncompleteRecordingsSectionProps {
  readonly incompleteList: readonly RecordingFileEntry[];
  readonly shownIncomplete: readonly RecordingFileEntry[];
  readonly recordingsChannelFilter: string;
  readonly selectedCount: number;
  readonly mergingRecordingKey: string | undefined;
  readonly deletingRecordingKey: string | undefined;
  readonly selectedIncompleteFilenames: Set<string>;
  readonly onToggleIncompleteMergeSelection: (filename: string) => void;
  readonly onRequestProcessIncompleteFiles: (channelLogin: string) => void;
  readonly onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
}

const EMPTY_LENGTH = 0;

export const IncompleteRecordingsSection = ({
  incompleteList,
  shownIncomplete,
  recordingsChannelFilter,
  selectedCount,
  mergingRecordingKey,
  deletingRecordingKey,
  selectedIncompleteFilenames,
  onToggleIncompleteMergeSelection,
  onRequestProcessIncompleteFiles,
  onRequestDeleteRecordingFile,
}: IncompleteRecordingsSectionProps): ReactElement => (
    <section className="recordings-section">
      <IncompleteSectionHeader
        incompleteCount={incompleteList.length}
        recordingsChannelFilter={recordingsChannelFilter}
        shownIncompleteLength={shownIncomplete.length}
        selectedCount={selectedCount}
        mergingRecordingKey={mergingRecordingKey}
        onRequestProcessIncompleteFiles={onRequestProcessIncompleteFiles}
      />
      {incompleteList.length === EMPTY_LENGTH ? (
        <p className="ui-muted section-empty">No incomplete files.</p>
      ) : (
        <ul className="recordings-list">
          {shownIncomplete.map((file) => (
            <IncompleteRecordingRow
              key={file.path_display}
              file={file}
              deletingRecordingKey={deletingRecordingKey}
              mergingRecordingKey={mergingRecordingKey}
              selectedIncompleteFilenames={selectedIncompleteFilenames}
              recordingsChannelFilter={recordingsChannelFilter}
              onToggleIncompleteMergeSelection={onToggleIncompleteMergeSelection}
              onRequestDeleteRecordingFile={onRequestDeleteRecordingFile}
            />
          ))}
        </ul>
      )}
    </section>
  );
