import type { ReactElement, ReactNode } from 'react';

interface MediaRowProps {
  extraClass?: string;
  meta?: ReactNode;
  onClick: () => void;
  title: string;
  visual: ReactNode;
}

export function MediaRow({
  extraClass = '',
  meta,
  onClick,
  title,
  visual,
}: MediaRowProps): ReactElement {
  return (
    <button
      type="button"
      className={`ui-card ui-card-interactive ui-media-row ${extraClass}`}
      onClick={onClick}
    >
      <div className="ui-media-visual">{visual}</div>
      <div className="ui-media-main">
        <span className="ui-media-title" title={title}>
          {title}
        </span>
        {meta}
      </div>
    </button>
  );
}

interface MediaRowAvatarProps {
  alt?: string;
  fallbackInitial: string;
  size?: string;
  src?: string | undefined;
}

export function MediaRowAvatar({
  alt,
  fallbackInitial,
  size = '74px',
  src,
}: MediaRowAvatarProps): ReactElement {
  const style = {
    width: size,
    height: size,
    borderRadius: '50%',
  };

  if (!src) {
    return (
      <div
        className="ui-avatar ui-avatar-fallback"
        style={style}
        role="img"
        aria-label={alt ?? fallbackInitial}
      >
        {fallbackInitial}
      </div>
    );
  }

  return (
    <img
      className="ui-avatar"
      style={style}
      src={src}
      alt={alt ?? fallbackInitial}
      loading="lazy"
    />
  );
}

interface MediaRowMetaProps {
  children: ReactNode;
}

export function MediaRowMeta({ children }: MediaRowMetaProps): ReactElement {
  return <span className="ui-media-meta">{children}</span>;
}
