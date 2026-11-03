import { describe, it, expect } from 'vitest';
import { renderToString } from 'react-dom/server';
import { RuntimeRuleIcon, isRuntimeRule } from '../RuntimeRuleIcon';

describe('isRuntimeRule helper', () => {
  it('identifies runtime rule filenames correctly (positive cases)', () => {
    expect(isRuntimeRule('runtime.md')).toBe(true);
    expect(isRuntimeRule('runtime')).toBe(true);
    expect(isRuntimeRule('RUNTIME.MD')).toBe(true);
    expect(isRuntimeRule('Runtime.md')).toBe(true);
    expect(isRuntimeRule('Runtime')).toBe(true);
    expect(isRuntimeRule('  runtime.md  ')).toBe(true);
    expect(isRuntimeRule('  runtime  ')).toBe(true);
  });

  it('rejects non-runtime rule filenames (negative cases)', () => {
    expect(isRuntimeRule('architecture.md')).toBe(false);
    expect(isRuntimeRule('backend_map.md')).toBe(false);
    expect(isRuntimeRule('frontend_map.md')).toBe(false);
    expect(isRuntimeRule('runtime.txt')).toBe(false);
    expect(isRuntimeRule('run')).toBe(false);
    expect(isRuntimeRule('')).toBe(false);
    expect(isRuntimeRule('   ')).toBe(false);
    expect(isRuntimeRule(null)).toBe(false);
    expect(isRuntimeRule(undefined)).toBe(false);
  });
});

describe('RuntimeRuleIcon component', () => {
  it('renders SVG with correct chip and prompt glyph markup', () => {
    const html = renderToString(<RuntimeRuleIcon />);

    // SVG root
    expect(html).toContain('<svg');
    expect(html).toContain('viewBox="0 0 24 24"');
    expect(html).toContain('xmlns="http://www.w3.org/2000/svg"');

    // Default classes
    expect(html).toContain('h-3.5 w-3.5 shrink-0');

    // Outer chip rectangle
    expect(html).toContain('<rect');
    expect(html).toContain('x="4"');
    expect(html).toContain('y="4"');
    expect(html).toContain('width="16"');
    expect(html).toContain('height="16"');

    // Silicon connection pins
    expect(html).toContain('<line');

    // Embedded prompt glyph: chevron path and cursor line
    expect(html).toContain('<path');
    expect(html).toContain('d="M8.5 9.5L11.5 12L8.5 14.5"');
    expect(html).toContain('x1="13.5"');
    expect(html).toContain('y1="14.5"');
  });

  it('propagates custom className to svg root', () => {
    const customClass = 'h-4 w-4 text-indigo-500 test-chip-class';
    const html = renderToString(<RuntimeRuleIcon className={customClass} />);

    expect(html).toContain(customClass);
    expect(html).not.toContain('h-3.5 w-3.5 shrink-0');
  });
});
