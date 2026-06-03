import * as React from 'react';
import { DecoratorNode } from 'lexical';
import type { JSX } from 'react';
import type { EditorConfig, LexicalEditor, NodeKey } from 'lexical';

export class EmoteNode extends DecoratorNode<JSX.Element> {
  __code: string;
  __imageUrl: string;

  static getType(): string {
    return 'emote';
  }

  static clone(node: EmoteNode): EmoteNode {
    return new EmoteNode(node.__code, node.__imageUrl, node.__key);
  }

  constructor(code: string, imageUrl: string, key?: NodeKey) {
    super(key);
    this.__code = code;
    this.__imageUrl = imageUrl;
  }

  createDOM(_config: EditorConfig): HTMLElement {
    const span = document.createElement('span');
    span.contentEditable = 'false';
    span.className = 'lexical-emote-node';
    span.style.display = 'inline-block';
    span.style.verticalAlign = 'middle';
    span.style.margin = '0 2px';
    return span;
  }

  updateDOM(): boolean {
    return false;
  }

  decorate(_editor: LexicalEditor): JSX.Element {
    return React.createElement('img', {
      src: this.__imageUrl,
      alt: this.__code,
      title: this.__code,
      className: 'lexical-emote-image',
      style: {
        display: 'inline-block',
        height: '1.35em',
        width: 'auto',
        verticalAlign: 'middle',
      },
    });
  }

  getTextContent(): string {
    return this.__code;
  }

  getCode(): string {
    return this.__code;
  }

  getImageUrl(): string {
    return this.__imageUrl;
  }

  static importJSON(serializedNode: {
    code: string;
    imageUrl: string;
    type: string;
    version: number;
  }): EmoteNode {
    return new EmoteNode(serializedNode.code, serializedNode.imageUrl);
  }

  exportJSON(): {
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

export function $createEmoteNode(code: string, imageUrl: string): EmoteNode {
  return new EmoteNode(code, imageUrl);
}

export function $isEmoteNode(node: unknown): node is EmoteNode {
  return node instanceof EmoteNode;
}
