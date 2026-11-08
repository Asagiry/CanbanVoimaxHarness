import { describe, it, expect } from 'vitest';
import { getRootTaskId, isSameFamily, isFamilyHighlighted } from '../cardFamily';

describe('cardFamily', () => {
  it('getRootTaskId returns parentId when present', () => {
    expect(getRootTaskId({ id: 'CVH-066.1', parentId: 'CVH-066' })).toBe('CVH-066');
  });

  it('getRootTaskId falls back to id split for child without parentId', () => {
    expect(getRootTaskId({ id: 'CVH-066.2' })).toBe('CVH-066');
  });

  it('getRootTaskId returns own id for root task', () => {
    expect(getRootTaskId({ id: 'CVH-066' })).toBe('CVH-066');
  });

  it('isSameFamily matches root and siblings', () => {
    expect(isSameFamily('CVH-066', 'CVH-066.1')).toBe(true);
    expect(isSameFamily('CVH-066.1', 'CVH-066.2')).toBe(true);
    expect(isSameFamily('CVH-066.2', 'CVH-066')).toBe(true);
  });

  it('isSameFamily rejects cross-family ids', () => {
    expect(isSameFamily('CVH-066.1', 'CVH-063')).toBe(false);
    expect(isSameFamily('CVH-066', 'CVH-063.1')).toBe(false);
  });

  it('isSameFamily is case-insensitive', () => {
    expect(isSameFamily('cvh-066.1', 'CVH-066.2')).toBe(true);
    expect(isSameFamily('CVH-066', 'cvh-066')).toBe(true);
  });

  it('isFamilyHighlighted highlights whole family for any hovered member', () => {
    const family = [
      { id: 'CVH-066' },
      { id: 'CVH-066.1', parentId: 'CVH-066' },
      { id: 'CVH-066.2', parentId: 'CVH-066' },
    ];
    for (const hovered of ['CVH-066', 'CVH-066.1', 'CVH-066.2']) {
      for (const task of family) {
        expect(isFamilyHighlighted(task, hovered)).toBe(true);
      }
    }
  });

  it('isFamilyHighlighted rejects unrelated task and null hover', () => {
    expect(isFamilyHighlighted({ id: 'CVH-063' }, 'CVH-066.1')).toBe(false);
    expect(isFamilyHighlighted({ id: 'CVH-066.1' }, null)).toBe(false);
    expect(isFamilyHighlighted({ id: 'CVH-066.1' }, undefined)).toBe(false);
  });
});
