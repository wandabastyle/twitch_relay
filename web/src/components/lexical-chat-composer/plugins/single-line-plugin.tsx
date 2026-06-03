import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $createParagraphNode,
  $createTextNode,
  $getRoot,
  $getSelection,
  COMMAND_PRIORITY_HIGH,
  INSERT_LINE_BREAK_COMMAND,
  INSERT_PARAGRAPH_COMMAND,
  PASTE_COMMAND,
} from 'lexical';
import { useEffect, type ReactElement } from 'react';

const EMPTY_SIZE = 0;
const MAX_LENGTH = 500;

export const SingleLinePlugin = (): ReactElement | null => {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    const removeLineBreak = editor.registerCommand(
      INSERT_LINE_BREAK_COMMAND,
      () => true,
      COMMAND_PRIORITY_HIGH,
    );

    const removeParagraphBreak = editor.registerCommand(
      INSERT_PARAGRAPH_COMMAND,
      () => true,
      COMMAND_PRIORITY_HIGH,
    );

    const normalizePaste = editor.registerCommand(
      PASTE_COMMAND,
      (event: ClipboardEvent) => {
        event.preventDefault();
        const text = event.clipboardData?.getData('text/plain') ?? '';
        const normalized = text.replaceAll(/\r?\n/g, ' ').replaceAll(/\s+/g, ' ');

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

    const cleanup = (): void => {
      removeLineBreak();
      removeParagraphBreak();
      normalizePaste();
    };

    return cleanup;
  }, [editor]);

  return null;
};

export const MaxLengthPlugin = (): ReactElement | null => {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    const unregister = editor.registerUpdateListener(
      ({ editorState, dirtyElements, dirtyLeaves }) => {
        if (dirtyElements.size === EMPTY_SIZE && dirtyLeaves.size === EMPTY_SIZE) {
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
                  paragraph.append($createTextNode(currentText.slice(EMPTY_SIZE, MAX_LENGTH)));
                  rootNode.clear();
                  rootNode.append(paragraph);
                }
              });
            });
          }
        });
      },
    );

    return unregister;
  }, [editor]);

  return null;
};
