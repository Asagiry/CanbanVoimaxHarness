import { useEffect, useState } from 'react';
import { Minus, Square, Copy, X } from 'lucide-react';

export async function toggleMaximizeWindow(): Promise<boolean> {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const win = getCurrentWindow();
    await win.toggleMaximize();
    return await win.isMaximized();
  } catch (e) {
    console.info('Window toggle maximize not available in web mode', e);
    return false;
  }
}

async function minimizeWindow() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().minimize();
  } catch (e) {
    console.info('Window minimize not available in web mode', e);
  }
}

async function closeWindow() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } catch (e) {
    console.info('Window close not available in web mode', e);
  }
}

// Shared frameless window controls (h-10) for the main topbar and the chat header.
export default function WindowControls() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    let isMounted = true;
    let unlistenResize: (() => void) | undefined;

    const setupWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const win = getCurrentWindow();
        const max = await win.isMaximized();
        if (!isMounted) return;
        setIsMaximized(max);

        const unlisten = await win.onResized(async () => {
          try {
            const currentMax = await win.isMaximized();
            if (isMounted) {
              setIsMaximized(currentMax);
            }
          } catch {
            // ignore non-tauri errors
          }
        });

        if (!isMounted) {
          unlisten();
        } else {
          unlistenResize = unlisten;
        }
      } catch {
        // Fallback for web preview mode
      }
    };

    void setupWindow();

    return () => {
      isMounted = false;
      if (unlistenResize) {
        unlistenResize();
      }
    };
  }, []);

  return (
    <div className="flex items-center h-full" data-no-drag>
      {/* Minimize Button */}
      <button
        type="button"
        data-no-drag
        onMouseDown={(e) => e.stopPropagation()}
        onClick={() => void minimizeWindow()}
        className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:text-[#18181b] hover:bg-[#f4f4f5] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] dark:hover:bg-[#161b22] active:bg-[#e4e4e7] dark:active:bg-[#21262d] transition-colors cursor-pointer select-none"
        title="Свернуть"
        aria-label="Свернуть окно"
      >
        <Minus className="h-3.5 w-3.5" strokeWidth={1.75} />
      </button>

      {/* Maximize / Restore Button */}
      <button
        type="button"
        data-no-drag
        onMouseDown={(e) => e.stopPropagation()}
        onClick={() => void toggleMaximizeWindow().then(setIsMaximized)}
        className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:text-[#18181b] hover:bg-[#f4f4f5] dark:text-[#8b949e] dark:hover:text-[#f0f6fc] dark:hover:bg-[#161b22] active:bg-[#e4e4e7] dark:active:bg-[#21262d] transition-colors cursor-pointer select-none"
        title={isMaximized ? 'Восстановить' : 'Развернуть'}
        aria-label={isMaximized ? 'Восстановить окно' : 'Развернуть окно'}
      >
        {isMaximized ? (
          <Copy className="h-3 w-3 -scale-y-100" strokeWidth={1.75} />
        ) : (
          <Square className="h-3 w-3" strokeWidth={1.75} />
        )}
      </button>

      {/* Close Button: Signature Red on Hover */}
      <button
        type="button"
        data-no-drag
        onMouseDown={(e) => e.stopPropagation()}
        onClick={() => void closeWindow()}
        className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:bg-rose-600 hover:text-white active:bg-rose-700 transition-colors cursor-pointer select-none"
        title="Закрыть"
        aria-label="Закрыть окно"
      >
        <X className="h-3.5 w-3.5" strokeWidth={2} />
      </button>
    </div>
  );
}
