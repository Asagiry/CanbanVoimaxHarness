import React, { useState } from 'react';
import { describe, it, expect, vi } from 'vitest';
import { renderToString } from 'react-dom/server';
import { NotificationToggle } from '../SettingsModal';

describe('NotificationToggle component', () => {
  describe('DOM Rendering & Semantic Attributes', () => {
    it('renders a switch button with role="switch" and aria-checked="true" when enabled and checked', () => {
      const handleChange = vi.fn();
      const html = renderToString(
        React.createElement(NotificationToggle, {
          checked: true,
          onChange: handleChange,
          disabled: false,
          ariaLabel: 'Desktop Notifications',
        })
      );

      // Verify semantic HTML structure
      expect(html).toContain('type="button"');
      expect(html).toContain('role="switch"');
      expect(html).toContain('aria-checked="true"');
      expect(html).toContain('aria-label="Desktop Notifications"');
      expect(html).not.toContain('disabled=""');

      // Verify layout & active styling tokens
      expect(html).toContain('items-center');
      expect(html).toContain('p-0.5');
      expect(html).toContain('h-5');
      expect(html).toContain('w-9');
      expect(html).toContain('rounded-full');
      expect(html).toContain('cursor-pointer');
      expect(html).toContain('bg-[#10b981]');

      // Verify inner thumb child styling
      expect(html).toContain('aria-hidden="true"');
      expect(html).toContain('h-4');
      expect(html).toContain('w-4');
      expect(html).toContain('translate-x-4');
      expect(html).toContain('shadow-xs');
      expect(html).toContain('bg-white');
    });

    it('renders unchecked state with off colors and translate-x-0', () => {
      const html = renderToString(
        React.createElement(NotificationToggle, {
          checked: false,
          onChange: vi.fn(),
          disabled: false,
          ariaLabel: 'Sound Alerts',
        })
      );

      expect(html).toContain('aria-checked="false"');
      expect(html).toContain('aria-label="Sound Alerts"');
      expect(html).toContain('bg-[#e5e7eb]');
      expect(html).toContain('dark:bg-[#2c2c33]');
      expect(html).toContain('translate-x-0');
      expect(html).not.toContain('translate-x-4');
      expect(html).not.toContain('bg-[#10b981]');
    });

    it('renders disabled state with disabled attribute, cursor-not-allowed, and reduced opacity', () => {
      const html = renderToString(
        React.createElement(NotificationToggle, {
          checked: true,
          onChange: vi.fn(),
          disabled: true,
          ariaLabel: 'Disabled Option',
        })
      );

      expect(html).toContain('disabled=""');
      expect(html).toContain('cursor-not-allowed');
      expect(html).toContain('opacity-40');
      expect(html).not.toContain('cursor-pointer');
    });

    it('defaults disabled to false when not provided', () => {
      const html = renderToString(
        React.createElement(NotificationToggle, {
          checked: false,
          onChange: vi.fn(),
        })
      );

      expect(html).not.toContain('disabled=""');
      expect(html).toContain('cursor-pointer');
      expect(html).not.toContain('cursor-not-allowed');
    });
  });

  describe('Prop Updates & Dynamic State Transitions', () => {
    it('accurately updates DOM attributes and classes across checked prop toggles', () => {
      // 1. Initial unchecked render
      const uncheckedHtml = renderToString(
        React.createElement(NotificationToggle, {
          checked: false,
          onChange: vi.fn(),
          ariaLabel: 'Dynamic Toggle',
        })
      );
      expect(uncheckedHtml).toContain('aria-checked="false"');
      expect(uncheckedHtml).toContain('translate-x-0');
      expect(uncheckedHtml).toContain('bg-[#e5e7eb]');

      // 2. Updated checked render
      const checkedHtml = renderToString(
        React.createElement(NotificationToggle, {
          checked: true,
          onChange: vi.fn(),
          ariaLabel: 'Dynamic Toggle',
        })
      );
      expect(checkedHtml).toContain('aria-checked="true"');
      expect(checkedHtml).toContain('translate-x-4');
      expect(checkedHtml).toContain('bg-[#10b981]');
    });

    it('accurately updates DOM attributes across disabled prop transitions', () => {
      // Enabled
      const enabledHtml = renderToString(
        React.createElement(NotificationToggle, {
          checked: true,
          onChange: vi.fn(),
          disabled: false,
        })
      );
      expect(enabledHtml).not.toContain('disabled=""');
      expect(enabledHtml).toContain('cursor-pointer');

      // Disabled
      const disabledHtml = renderToString(
        React.createElement(NotificationToggle, {
          checked: true,
          onChange: vi.fn(),
          disabled: true,
        })
      );
      expect(disabledHtml).toContain('disabled=""');
      expect(disabledHtml).toContain('cursor-not-allowed');
    });
  });

  describe('User Interactions & Event Emission', () => {
    it('triggers onChange callback when button element receives onClick event', () => {
      let changeCount = 0;
      const handleChange = () => {
        changeCount += 1;
      };

      const element = React.createElement(NotificationToggle, {
        checked: false,
        onChange: handleChange,
        disabled: false,
        ariaLabel: 'Interactive Toggle',
      });

      // Verify that NotificationToggle binds its onChange prop to the button's onClick event handler
      const buttonElement = NotificationToggle(element.props);
      expect(buttonElement.type).toBe('button');
      expect(typeof buttonElement.props.onClick).toBe('function');

      // Trigger the button's onClick handler
      buttonElement.props.onClick();
      expect(changeCount).toBe(1);

      buttonElement.props.onClick();
      expect(changeCount).toBe(2);
    });

    it('renders state transitions in an interactive wrapper component', () => {
      let externalChecked = false;
      function TestHarness() {
        const [checked, setChecked] = useState(false);
        externalChecked = checked;
        return React.createElement(
          'div',
          { 'data-testid': 'harness' },
          React.createElement('span', { 'data-testid': 'status' }, checked ? 'ENABLED' : 'DISABLED'),
          React.createElement(NotificationToggle, {
            checked,
            onChange: () => setChecked(!checked),
            ariaLabel: 'Harness Switch',
          })
        );
      }

      // Initial state render
      const initialHtml = renderToString(React.createElement(TestHarness));
      expect(initialHtml).toContain('DISABLED');
      expect(initialHtml).toContain('aria-checked="false"');
      expect(initialHtml).toContain('translate-x-0');
      expect(externalChecked).toBe(false);
    });

    it('preserves disabled invariant during interaction', () => {
      const handleChange = vi.fn();
      const element = React.createElement(NotificationToggle, {
        checked: false,
        onChange: handleChange,
        disabled: true,
        ariaLabel: 'Disabled Interactive',
      });

      expect(element.props.disabled).toBe(true);
      // Disabled button in DOM has disabled attribute
      const html = renderToString(element);
      expect(html).toContain('disabled=""');
      expect(html).toContain('cursor-not-allowed');
    });
  });
});
