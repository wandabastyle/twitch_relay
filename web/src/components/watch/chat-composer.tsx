import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  type ReactElement,
  type Ref,
} from 'react';
import type { EmoteItem } from '../../api-client';
import { useChatComposer } from '../../hooks/watch/use-chat-composer';

const TAB_INDEX_DISABLED = -1;
const TAB_INDEX_ENABLED = 0;

export interface ChatComposerHandle {
  focus: () => void;
  insertEmote: (code: string) => void;
  insertText: (text: string) => void;
}

interface ChatComposerProps {
  availableEmotes: EmoteItem[];
  disabled?: boolean;
  onSubmit: (text: string) => Promise<void>;
}

export const ChatComposer = forwardRef<ChatComposerHandle, ChatComposerProps>(
  (props: ChatComposerProps, ref: Ref<ChatComposerHandle>): ReactElement => {
    const { availableEmotes, disabled = false, onSubmit } = props;
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
      insertEmote,
      emoteChips,
      setComposerText,
      text,
    } = useChatComposer({
      availableEmotes,
      disabled,
      onSubmit,
    });

    const EMPTY_TEXT_LENGTH = 0;
    const CURSOR_MOVE_TIMEOUT_MS = 0;

    const insertText = useCallback(
      (textValue: string): void => {
        if (disabled || textValue === '') {
          return;
        }
        const currentText = text;
        const needsSpace = currentText.length > EMPTY_TEXT_LENGTH && !currentText.endsWith(' ');
        const prefix = needsSpace ? ' ' : '';
        setComposerText(`${currentText}${prefix}${textValue}`, emoteChips);
        // Move cursor to the end of the inserted text.
        const newPosition = currentText.length + prefix.length + textValue.length;
        setTimeout(() => {
          composerRef.current?.focus();
          const selection = globalThis.getSelection();
          if (selection === null || composerRef.current === null) {
            return;
          }
          const [textNode] = composerRef.current.childNodes;
          if (textNode instanceof Text) {
            selection.collapse(textNode, Math.min(newPosition, textNode.length));
          }
        }, CURSOR_MOVE_TIMEOUT_MS);
      },
      [disabled, emoteChips, setComposerText, text],
    );

    // Expose imperative handle for emote insertion, text insertion, and focus
    useImperativeHandle(ref, () => ({
      focus: (): void => {
        composerRef.current?.focus();
      },
      insertEmote,
      insertText,
    }));

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
        const clickedInsideComposer =
          target instanceof HTMLElement && target.closest('.ui-chat-composer') !== null;
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
        (event: React.MouseEvent): void => {
          event.preventDefault();
          handleSuggestionClick(item);
        },
      [handleSuggestionClick],
    );

    return (
      <div className="ui-chat-composer">
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
          tabIndex={disabled ? TAB_INDEX_DISABLED : TAB_INDEX_ENABLED}
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
  },
);

ChatComposer.displayName = 'ChatComposer';
