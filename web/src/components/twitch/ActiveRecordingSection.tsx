import type { ReactElement } from 'react';
import type { ActiveRecording } from '../../api-client/types';

interface ActiveRecordingsSectionProps {
  readonly activeList: ActiveRecording[];
  readonly shownActive: ActiveRecording[];
}

export const ActiveRecordingsSection = ({
  activeList,
  shownActive,
}: ActiveRecordingsSectionProps): ReactElement => (
  <section className="recordings-section">
    <h2>Active ({activeList.length})</h2>
    {activeList.length === 0 ? (
      <p className="ui-muted section-empty">No active recordings right now.</p>
    ) : (
      <ul className="recordings-list">
        {shownActive.map((recording) => (
          <li key={recording.channel_login}>
            <span className="entry-main">{recording.channel_login}</span>
            <span className="entry-meta">
              {recording.mode} · {recording.quality}
            </span>
          </li>
        ))}
      </ul>
    )}
  </section>
);
