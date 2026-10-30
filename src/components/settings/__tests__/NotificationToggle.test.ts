import { describe, it, expect } from 'vitest';
import { NotificationToggle } from '../SettingsModal';

describe('NotificationToggle component', () => {
  it('renders switch button with correct attributes when enabled and checked', () => {
    let clicked = false;
    const el = NotificationToggle({
      checked: true,
      onChange: () => { clicked = true; },
      disabled: false,
      ariaLabel: 'Test Switch',
    });

    expect(el.type).toBe('button');
    expect(el.props.role).toBe('switch');
    expect(el.props['aria-checked']).toBe(true);
    expect(el.props['aria-label']).toBe('Test Switch');
    expect(el.props.disabled).toBe(false);

    // Verify centering and alignment styles
    expect(el.props.className).toContain('items-center');
    expect(el.props.className).toContain('p-0.5');
    expect(el.props.className).toContain('h-5');
    expect(el.props.className).toContain('w-9');
    expect(el.props.className).toContain('rounded-full');
    expect(el.props.className).toContain('cursor-pointer');
    expect(el.props.className).toContain('bg-[#10b981]');

    // Verify thumb child styling
    const thumb = el.props.children;
    expect(thumb.type).toBe('span');
    expect(thumb.props['aria-hidden']).toBe('true');
    expect(thumb.props.className).toContain('block');
    expect(thumb.props.className).toContain('h-4');
    expect(thumb.props.className).toContain('w-4');
    expect(thumb.props.className).toContain('translate-x-4');
    expect(thumb.props.className).toContain('shadow-xs');

    el.props.onClick();
    expect(clicked).toBe(true);
  });

  it('renders unchecked state with off colors and translate-x-0', () => {
    const el = NotificationToggle({
      checked: false,
      onChange: () => {},
      disabled: false,
      ariaLabel: 'Unchecked Switch',
    });

    expect(el.props['aria-checked']).toBe(false);
    expect(el.props.className).toContain('bg-[#e5e7eb]');
    expect(el.props.className).toContain('dark:bg-[#30363d]');

    const thumb = el.props.children;
    expect(thumb.props.className).toContain('translate-x-0');
  });

  it('renders disabled state with cursor-not-allowed and reduced opacity', () => {
    const el = NotificationToggle({
      checked: true,
      onChange: () => {},
      disabled: true,
      ariaLabel: 'Disabled Switch',
    });

    expect(el.props.disabled).toBe(true);
    expect(el.props.className).toContain('cursor-not-allowed');
    expect(el.props.className).toContain('opacity-40');
    expect(el.props.className).not.toContain('cursor-pointer');
  });
});
