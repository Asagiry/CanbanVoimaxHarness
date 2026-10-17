import { create } from 'zustand';

export type Theme = 'light' | 'dark';

const STORAGE_KEY = 'cvh:theme';

function getInitialTheme(): Theme {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'dark' || saved === 'light') {
      return saved;
    }
  } catch {
    // localStorage not accessible
  }
  return 'light';
}

function applyThemeToDocument(theme: Theme) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  if (theme === 'dark') {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }
}

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  toggleThemeWithAnimation: (event?: { clientX?: number; clientY?: number }) => void;
}

export const useThemeStore = create<ThemeState>((set, get) => {
  const initialTheme = getInitialTheme();
  applyThemeToDocument(initialTheme);

  return {
    theme: initialTheme,
    setTheme: (newTheme: Theme) => {
      try {
        localStorage.setItem(STORAGE_KEY, newTheme);
      } catch {
        // ignore
      }
      applyThemeToDocument(newTheme);
      set({ theme: newTheme });
    },
    toggleTheme: () => {
      const nextTheme = get().theme === 'light' ? 'dark' : 'light';
      get().setTheme(nextTheme);
    },
    toggleThemeWithAnimation: (event) => {
      const toggle = () => {
        get().toggleTheme();
      };

      if (typeof window === 'undefined' || typeof document === 'undefined') {
        toggle();
        return;
      }

      const doc = document as Document & {
        startViewTransition?: (callback: () => void | Promise<void>) => {
          ready: Promise<void>;
          finished: Promise<void>;
          updateCallbackDone: Promise<void>;
        };
      };

      if (!doc.startViewTransition) {
        toggle();
        return;
      }

      const x = event && typeof event.clientX === 'number'
        ? event.clientX
        : window.innerWidth / 2;
      const y = event && typeof event.clientY === 'number'
        ? event.clientY
        : window.innerHeight / 2;

      const endRadius = Math.hypot(
        Math.max(x, window.innerWidth - x),
        Math.max(y, window.innerHeight - y)
      );

      document.documentElement.style.setProperty('--theme-x', x + 'px');
      document.documentElement.style.setProperty('--theme-y', y + 'px');
      document.documentElement.style.setProperty('--theme-radius', endRadius + 'px');

      doc.startViewTransition(() => {
        toggle();
      });
    },
  };
});
