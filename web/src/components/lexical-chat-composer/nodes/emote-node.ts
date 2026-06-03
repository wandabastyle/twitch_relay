import { DecoratorNode, type EditorConfig, type LexicalEditor, type NodeKey } from 'lexical';
import * as React from 'react';

const EMOTE_HEIGHT = '1.35em';
const IMPOSSIBLE_LENGTH = -1;

export class EmoteNode extends DecoratorNode<React.ReactElement> {
  private readonly __code: string;
  private readonly __imageUrl: string;

  public static override getType(): string {
    return 'emote';
  }

  public static override clone(node: EmoteNode): EmoteNode {
    return new EmoteNode(node.__code, node.__imageUrl, node.__key);
  }

  public constructor(code: string, imageUrl: string, key?: NodeKey) {
    super(key);
    this.__code = code;
    this.__imageUrl = imageUrl;
  }

  public override createDOM(_config: EditorConfig): HTMLElement {
    const span = document.createElement('span');
    span.contentEditable = 'false';
    span.className = 'lexical-emote-node';
    span.dataset.emoteCode = this.__code;
    span.style.display = 'inline-block';
    span.style.margin = '0 2px';
    span.style.verticalAlign = 'middle';
    return span;
  }

  public override updateDOM(): boolean {
    return this.__code.length === IMPOSSIBLE_LENGTH;
  }

  public override decorate(_editor: LexicalEditor): React.ReactElement {
    return React.createElement('img', {
      alt: this.__code,
      className: 'lexical-emote-image',
      src: this.__imageUrl,
      style: {
        display: 'inline-block',
        height: EMOTE_HEIGHT,
        verticalAlign: 'middle',
        width: 'auto',
      },
      title: this.__code,
    });
  }

  public override getTextContent(): string {
    return this.__code;
  }

  public getCode(): string {
    return this.__code;
  }

  public getImageUrl(): string {
    return this.__imageUrl;
  }

  public static override importJSON(serializedNode: {
    code: string;
    imageUrl: string;
    type: string;
    version: number;
  }): EmoteNode {
    return new EmoteNode(serializedNode.code, serializedNode.imageUrl);
  }

  public override exportJSON(): {
    code: string;
    imageUrl: string;
    type: string;
    version: number;
  } {
    return {
      code: this.__code,
      imageUrl: this.__imageUrl,
      type: 'emote',
      version: 1,
    };
  }
}

export const $createEmoteNode = (code: string, imageUrl: string): EmoteNode =>
  new EmoteNode(code, imageUrl);

export const $isEmoteNode = (node: unknown): node is EmoteNode => node instanceof EmoteNode;
