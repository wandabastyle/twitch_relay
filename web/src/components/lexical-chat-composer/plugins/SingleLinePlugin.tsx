import { useEffect } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $createParagraphNode,
  $createTextNode,
  $getSelection,
  $getRoot,
  COMMAND_PRIORITY_HIGH,
  INSERT_LINE_BREAK_COMMAND,
  INSERT_PARAGRAPH_COMMAND,
  PASTE_COMMAND,
} from 'lexical';

const MAX_LENGTH = 500;

export function SingleLinePlugin(): null {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    // Prevent Enter key from creating new lines
    const removeLineBreak = editor.registerCommand(
      INSERT_LINE_BREAK_COMMAND,
      () => {
        return true;
      },
      COMMAND_PRIORITY_HIGH,
    );

    // Prevent Shift+Enter or other paragraph insertion
    const removeParagraphBreak = editor.registerCommand(
      INSERT_PARAGRAPH_COMMAND,
      () => {
        return true;
      },
      COMMAND_PRIORITY_HIGH,
    );

    // Normalize pasted text - replace newlines with spaces
    const normalizePaste = editor.registerCommand(
      PASTE_COMMAND,
      (event: ClipboardEvent) => {
        event.preventDefault();
        const text = event.clipboardData?.getData('text/plain') ?? '';
        const normalized = text.replace(/\r?\n/g, ' ').replace(/\s+/g, ' ');

        editor.update(() => {
          const selection = $getSelection();
          if (selection !== null) {
            selection.insertText(normalized);
          }
        });

        return true;
      },
      COMMAND_PRIORITY_HIGH,
    );

    return () => {
      removeLineBreak();
      removeParagraphBreak();
      normalizePaste();
    };
  }, [editor]);

  return null;
}

export function MaxLengthPlugin(): null {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    return editor.registerUpdateListener(({ editorState, dirtyElements, dirtyLeaves }) => {
      if (dirtyElements.size === 0 && dirtyLeaves.size === 0) {
        return;
      }

      editorState.read(() => {
        const root = $getRoot();
        const text = root.getTextContent();

        if (text.length > MAX_LENGTH) {
          queueMicrotask(() => {
            editor.update(() => {
              const rootNode = $getRoot();
              const currentText = rootNode.getTextContent();
              if (currentText.length > MAX_LENGTH) {
                const paragraph = $createParagraphNode();
                paragraph.append($createTextNode(currentText.slice(0, MAX_LENGTH)));
                rootNode.clear();
                rootNode.append(paragraph);
              }
            });
          });
        }
      });
    });
  }, [editor]);

  return null;
}
