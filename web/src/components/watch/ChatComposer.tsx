import { useCallback, useEffect, useRef, type ReactElement } from 'react';
import { useChatComposer } from '../../hooks/watch/useChatComposer';
import type { EmoteItem } from '../../api-client';

interface ChatComposerProps {
  availableEmotes: EmoteItem[];
  disabled?: boolean;
  onSubmit: (text: string) => void;
}

export const ChatComposer = ({
  availableEmotes,
  disabled = false,
  onSubmit,
}: ChatComposerProps): ReactElement => {
  const containerRef = useRef<HTMLDivElement>(null);
  const {
    composerRef,
    suggestionsOpen,
    suggestionItems,
    suggestionIndex,
    previewOpen,
    previewUrl,
    previewPosition,
    handleInput,
    handlePaste,
    handleKeydown,
    handleSuggestionClick,
    closeSuggestions,
    clearPreview,
  } = useChatComposer({
    availableEmotes,
    disabled,
    onSubmit,
  });

  // Handle click outside to close suggestions
  useEffect(() => {
    const handleDocumentClick = (event: MouseEvent): void => {
      if (!suggestionsOpen) {
        return;
      }
      const { target } = event;
      if (!target) {
        return;
      }
      const clickedInsideComposer = (target as HTMLElement).closest('.ui-chat-composer') !== null;
      if (!clickedInsideComposer) {
        closeSuggestions();
      }
    };

    document.addEventListener('click', handleDocumentClick);
    return (): void => {
      document.removeEventListener('click', handleDocumentClick);
    };
  }, [suggestionsOpen, closeSuggestions]);

  // Cleanup preview timer on unmount
  useEffect(
    () => (): void => {
      clearPreview();
    },
    [clearPreview],
  );

  const onSuggestionMouseDown = useCallback(
    (item: EmoteItem) =>
      (e: React.MouseEvent): void => {
        e.preventDefault();
        handleSuggestionClick(item);
      },
    [handleSuggestionClick],
  );

  return (
    <div ref={containerRef} className="ui-chat-composer">
      {previewOpen && (
        <div
          className="ui-chat-emote-preview visible"
        style={{
          backgroundImage: `url('${previewUrl}')`,
          left: `${previewPosition.left}px`,
          top: `${previewPosition.top}px`,
        }}
        />
      )}
      <div
        ref={composerRef}
        className={`ui-chat-composer-input ${disabled ? 'is-disabled' : ''}`}
        contentEditable={!disabled}
        role="textbox"
        tabIndex={disabled ? -1 : 0}
        aria-label="Send a message"
        data-placeholder="Send a message"
        aria-disabled={disabled}
        onInput={handleInput}
        onPaste={handlePaste}
        onKeyDown={handleKeydown}
      />
      {suggestionsOpen && (
        <div className="ui-chat-suggestions ui-hide-scrollbar">
          {suggestionItems.map((item, index) => (
            <button
              key={item.id}
              type="button"
              className={`ui-chat-suggestion-item ${index === suggestionIndex ? 'active' : ''}`}
              onMouseDown={onSuggestionMouseDown(item)}
            >
              <img src={item.image_url} alt={item.code} loading="lazy" decoding="async" />
              <span>{item.code}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
