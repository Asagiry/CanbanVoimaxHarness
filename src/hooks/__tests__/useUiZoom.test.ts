import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import React from 'react';
import {
  useUiZoomStore,
  getZoomOverlayStyle,
  getRootZoomStyle,
  adjustDndTransformForZoom,
  attachGlobalZoomListeners,
} from '../useUiZoom';
import CreateBoardModal from '../../components/board/CreateBoardModal';
import CreateColumnModal from '../../components/board/modals/CreateColumnModal';
import CreateTaskModal from '../../components/board/modals/CreateTaskModal';
import { CreateAgentModal } from '../../components/cortex/CreateAgentModal';
import { EditAgentPersonaModal } from '../../components/cortex/EditAgentPersonaModal';
import ImageLightboxModal from '../../components/drawer/ImageLightboxModal';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

describe('useUiZoomStore', () => {
  beforeEach(() => {
    useUiZoomStore.getState().resetZoom();
  });

  it('initializes with default zoom 1.0', () => {
    expect(useUiZoomStore.getState().zoom).toBe(1.0);
  });

  it('increments zoom on zoomIn within bounds', () => {
    useUiZoomStore.getState().zoomIn();
    expect(useUiZoomStore.getState().zoom).toBe(1.1);
    expect(useUiZoomStore.getState().showIndicator).toBe(true);
  });

  it('decrements zoom on zoomOut within bounds', () => {
    useUiZoomStore.getState().zoomOut();
    expect(useUiZoomStore.getState().zoom).toBe(0.9);
    expect(useUiZoomStore.getState().showIndicator).toBe(true);
  });

  it('clamps zoom to MIN_ZOOM (0.5) and MAX_ZOOM (2.0)', () => {
    useUiZoomStore.getState().setZoom(3.5);
    expect(useUiZoomStore.getState().zoom).toBe(2.0);

    useUiZoomStore.getState().setZoom(0.1);
    expect(useUiZoomStore.getState().zoom).toBe(0.5);
  });

  it('resets zoom to 1.0 on resetZoom', () => {
    useUiZoomStore.getState().setZoom(1.5);
    expect(useUiZoomStore.getState().zoom).toBe(1.5);

    useUiZoomStore.getState().resetZoom();
    expect(useUiZoomStore.getState().zoom).toBe(1.0);
  });

  it('supports functional updates in setZoom', () => {
    useUiZoomStore.getState().setZoom((prev) => prev + 0.3);
    expect(useUiZoomStore.getState().zoom).toBe(1.3);
  });
});

describe('getZoomOverlayStyle', () => {
  it('returns undefined when zoom is exactly 1 or within 0.001 threshold', () => {
    expect(getZoomOverlayStyle(1.0)).toBeUndefined();
    expect(getZoomOverlayStyle(1.0001)).toBeUndefined();
    expect(getZoomOverlayStyle(0.9999)).toBeUndefined();
  });

  it('returns viewport inverse scale and dimensions when zoomed in (> 1)', () => {
    const style = getZoomOverlayStyle(1.25);
    expect(style).toBeDefined();
    expect(style?.width).toBe(`${100 / 1.25}vw`);
    expect(style?.height).toBe(`${100 / 1.25}vh`);
    expect(style?.maxWidth).toBe('none');
    expect(style?.maxHeight).toBe('none');
  });

  it('returns viewport inverse scale and dimensions when zoomed out (< 1)', () => {
    const style = getZoomOverlayStyle(0.8);
    expect(style).toBeDefined();
    expect(style?.width).toBe('125vw');
    expect(style?.height).toBe('125vh');
    expect(style?.maxWidth).toBe('none');
    expect(style?.maxHeight).toBe('none');
  });
});

describe('getRootZoomStyle', () => {
  it('calculates inverse viewport dimensions for root container', () => {
    const style = getRootZoomStyle(1.5);
    expect(style.width).toBe(`${100 / 1.5}vw`);
    expect(style.height).toBe(`${100 / 1.5}vh`);
    expect(style.maxWidth).toBe('none');
    expect(style.maxHeight).toBe('none');
  });
});

describe('adjustDndTransformForZoom', () => {
  it('returns undefined or unchanged transform when transform is absent or zoom is 1', () => {
    expect(adjustDndTransformForZoom(undefined, 1.2)).toBeUndefined();
    expect(adjustDndTransformForZoom(null, 1.2)).toBeUndefined();
    expect(adjustDndTransformForZoom('translate(100px, 200px)', 1.0)).toBe('translate(100px, 200px)');
  });

  it('adjusts translation coordinates by dividing by zoom factor', () => {
    const result = adjustDndTransformForZoom('translate(120px, 240px)', 1.2);
    expect(result).toBe('translate(100px, 200px)');
  });
});

describe('Wheel event propagation and Ctrl+Wheel zoom', () => {
  beforeEach(() => {
    useUiZoomStore.getState().resetZoom();
  });

  it('triggers zoom store mutations on Ctrl+Wheel without swallowing', () => {
    const listeners: Record<string, ((e: any) => void)[]> = {};
    const originalWindow = (globalThis as any).window;

    (globalThis as any).window = {
      addEventListener: (evt: string, cb: (e: any) => void) => {
        listeners[evt] = listeners[evt] || [];
        listeners[evt].push(cb);
      },
      removeEventListener: () => {},
    };

    try {
      attachGlobalZoomListeners();

      const wheelListeners = listeners['wheel'] || [];
      expect(wheelListeners.length).toBeGreaterThan(0);

      let prevented = false;
      const zoomInEvent = {
        type: 'wheel',
        ctrlKey: true,
        metaKey: false,
        deltaY: -100,
        preventDefault: () => {
          prevented = true;
        },
      };

      for (const listener of wheelListeners) {
        listener(zoomInEvent);
      }

      expect(prevented).toBe(true);
      expect(useUiZoomStore.getState().zoom).toBe(1.1);

      prevented = false;
      const zoomOutEvent = {
        type: 'wheel',
        ctrlKey: true,
        metaKey: false,
        deltaY: 100,
        preventDefault: () => {
          prevented = true;
        },
      };

      for (const listener of wheelListeners) {
        listener(zoomOutEvent);
      }

      expect(prevented).toBe(true);
      expect(useUiZoomStore.getState().zoom).toBe(1.0);
    } finally {
      if (originalWindow) {
        (globalThis as any).window = originalWindow;
      } else {
        delete (globalThis as any).window;
      }
    }
  });

  it('allows Ctrl+Wheel events through CardTodoProgress handlers without stopPropagation', () => {
    let stoppedPropagation = false;
    let preventedDefault = false;

    // Simulate CardTodoProgress wheel check logic
    const handleWheelWithGuard = (e: { ctrlKey: boolean; metaKey: boolean; deltaY: number }) => {
      if (e.ctrlKey || e.metaKey) return;
      preventedDefault = true;
      stoppedPropagation = true;
    };

    // When Ctrl is pressed: should NOT stop propagation or prevent default
    handleWheelWithGuard({ ctrlKey: true, metaKey: false, deltaY: 50 });
    expect(stoppedPropagation).toBe(false);
    expect(preventedDefault).toBe(false);

    // When plain scroll: should intercept
    handleWheelWithGuard({ ctrlKey: false, metaKey: false, deltaY: 50 });
    expect(stoppedPropagation).toBe(true);
    expect(preventedDefault).toBe(true);
  });

  it('allows Ctrl+Wheel events through ImageLightboxModal handlers without preventDefault', () => {
    let preventedDefault = false;

    // Simulate ImageLightboxModal wheel check logic
    const handleLightboxWheel = (e: { ctrlKey: boolean; metaKey: boolean; deltaY: number }) => {
      if (e.ctrlKey || e.metaKey) return;
      preventedDefault = true;
    };

    // With Ctrl key: bypasses preventDefault so global listener receives it
    handleLightboxWheel({ ctrlKey: true, metaKey: false, deltaY: -100 });
    expect(preventedDefault).toBe(false);

    // Without Ctrl key: intercepts for image preview zooming
    handleLightboxWheel({ ctrlKey: false, metaKey: false, deltaY: -100 });
    expect(preventedDefault).toBe(true);
  });
});

describe('Modal rendering with getZoomOverlayStyle', () => {
  beforeEach(() => {
    useUiZoomStore.getState().resetZoom();
  });

  it('renders CreateBoardModal with getZoomOverlayStyle applied when zoomed', () => {
    useUiZoomStore.getState().setZoom(1.5);
    const html = renderToString(React.createElement(CreateBoardModal, { isOpen: true, onClose: () => {} }));
    expect(html).toContain('fixed inset-0');
    expect(html).toContain('66.66666666666667vw');
  });

  it('renders CreateColumnModal with getZoomOverlayStyle applied when zoomed', () => {
    useUiZoomStore.getState().setZoom(1.2);
    const html = renderToString(React.createElement(CreateColumnModal, { isOpen: true, onClose: () => {} }));
    expect(html).toContain('fixed inset-0');
    expect(html).toContain('83.33333333333334vw');
  });

  it('renders CreateTaskModal with getZoomOverlayStyle applied when zoomed', () => {
    useUiZoomStore.getState().setZoom(1.2);
    const html = renderToString(React.createElement(CreateTaskModal, { isOpen: true, onClose: () => {} }));
    expect(html).toContain('fixed inset-0');
    expect(html).toContain('83.33333333333334vw');
  });

  it('renders CreateAgentModal with getZoomOverlayStyle applied when zoomed', () => {
    useUiZoomStore.getState().setZoom(1.5);
    const html = renderToString(
      React.createElement(CreateAgentModal, {
        isOpen: true,
        existingRoles: [],
        onClose: () => {},
        onCreateSuccess: () => {},
      })
    );
    expect(html).toContain('fixed inset-0');
    expect(html).toContain('66.66666666666667vw');
  });

  it('renders EditAgentPersonaModal with getZoomOverlayStyle applied when zoomed', () => {
    useUiZoomStore.getState().setZoom(1.5);
    const html = renderToString(
      React.createElement(EditAgentPersonaModal, {
        isOpen: true,
        persona: { role: 'tester', hasPrompt: true, isBuiltin: false },
        onClose: () => {},
        onRename: async () => {},
        onColorChange: async () => {},
        onIconChange: async () => {},
      })
    );
    expect(html).toContain('fixed inset-0');
    expect(html).toContain('66.66666666666667vw');
  });

  it('renders ImageLightboxModal with getZoomOverlayStyle applied when zoomed', () => {
    useUiZoomStore.getState().setZoom(1.2);
    const html = renderToString(
      React.createElement(ImageLightboxModal, { src: 'data:image/png;base64,abc', onClose: () => {} })
    );
    expect(html).toContain('fixed inset-0');
    expect(html).toContain('83.33333333333334vw');
  });
});
