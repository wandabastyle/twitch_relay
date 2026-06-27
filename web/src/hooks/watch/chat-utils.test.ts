import { describe, expect, it } from 'vitest';
import { readableSenderColor } from './chat-utils';

describe('readableSenderColor', () => {
  it('lightens black sender colors for dark chat backgrounds', () => {
    expect(readableSenderColor('#000000')).toBe('#8C8C8C');
  });

  it('preserves already readable sender colors', () => {
    expect(readableSenderColor('#00FF00')).toBe('#00FF00');
  });

  it('keeps non-hex color values unchanged', () => {
    expect(readableSenderColor('rebeccapurple')).toBe('rebeccapurple');
  });
});
