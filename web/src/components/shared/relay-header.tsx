import { ArrowLeftRight } from 'lucide-react';
import type { ReactElement, ReactNode } from 'react';

interface RelayHeaderProps {
  eyebrow: string;
  title: string;
  subtitleText?: string;
  subtitleSnippet?: ReactNode;
  onToggle: () => void;
  toggleLabel: string;
  children?: ReactNode;
}

export const RelayHeader = ({
  children,
  eyebrow,
  onToggle,
  subtitleSnippet,
  subtitleText,
  title,
  toggleLabel,
}: RelayHeaderProps): ReactElement => {
  return (
    <header className="relay-header">
      <div className="relay-header-title">
        <p className="relay-header-eyebrow">{eyebrow}</p>
        <button
          type="button"
          className="relay-header-button"
          onClick={onToggle}
          aria-label={toggleLabel}
          title={toggleLabel}
        >
          <h1>{title}</h1>
          <span className="relay-header-toggle-icon" aria-hidden="true">
            <ArrowLeftRight size={14} />
          </span>
        </button>
        {(subtitleText || subtitleSnippet) && (
          <p className="relay-header-subtitle">{subtitleText || subtitleSnippet}</p>
        )}
      </div>
      {children}
    </header>
  );
}
