import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';

interface RecordingBadgesProps {
  readonly file: RecordingFileEntry;
}

export const RecordingBadges = ({ file }: RecordingBadgesProps): ReactElement => {
  return (
    <div className="recording-badges">
      {file.processing_state === 'processing' && (
        <span className="badge badge-processing">Processing</span>
      )}
      {file.pinned && <span className="badge badge-pinned">Pinned</span>}
      {!file.has_hls && <span className="badge badge-repair">Needs repair</span>}
    </div>
  );
}
