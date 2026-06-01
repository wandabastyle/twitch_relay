import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';
import { CompletedRecordingRow } from './completed-recording-row';

interface CompletedRecordingsSectionProps {
  readonly completedList: readonly RecordingFileEntry[];
  readonly shownCompleted: readonly RecordingFileEntry[];
  readonly deletingRecordingKey: string | undefined;
  readonly pinningRecordingKey: string | undefined;
  readonly repairingRecordingKey: string | undefined;
  readonly onToggleRecordingPin: (file: RecordingFileEntry) => void;
  readonly onOpenRecordingPlayer: (file: RecordingFileEntry) => void;
  readonly onRepairRecording: (file: RecordingFileEntry) => void;
  readonly onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
}

const EMPTY_LENGTH = 0;

export const CompletedRecordingsSection = ({
  completedList,
  shownCompleted,
  deletingRecordingKey,
  pinningRecordingKey,
  repairingRecordingKey,
  onToggleRecordingPin,
  onOpenRecordingPlayer,
  onRepairRecording,
  onRequestDeleteRecordingFile,
}: CompletedRecordingsSectionProps): ReactElement => (
  <section className="recordings-section">
    <h2>Completed ({completedList.length})</h2>
    {completedList.length === EMPTY_LENGTH ? (
      <p className="ui-muted section-empty">No completed files yet.</p>
    ) : (
      <ul className="recordings-list">
        {shownCompleted.map((file) => (
          <CompletedRecordingRow
            key={file.path_display}
            file={file}
            deletingRecordingKey={deletingRecordingKey}
            pinningRecordingKey={pinningRecordingKey}
            repairingRecordingKey={repairingRecordingKey}
            onToggleRecordingPin={onToggleRecordingPin}
            onOpenRecordingPlayer={onOpenRecordingPlayer}
            onRepairRecording={onRepairRecording}
            onRequestDeleteRecordingFile={onRequestDeleteRecordingFile}
          />
        ))}
      </ul>
    )}
  </section>
);
