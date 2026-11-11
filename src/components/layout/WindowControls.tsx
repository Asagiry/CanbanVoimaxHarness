import { useEffect, useState } from 'react';
import { Minus, Square, Copy, X } from 'lucide-react';
import { useTranslation } from '../../stores/languageStore';

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
  const { t } = useTranslation();

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
          } catch (err) {
            console.debug('Window resize query skipped in web mode:', err);
          }
        });

        if (!isMounted) {
          unlisten();
        } else {
          unlistenResize = unlisten;
        }
      } catch (err) {
        console.debug('Window controls init skipped in web mode:', err);
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
        className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:text-[#18181b] hover:bg-[#f4f4f5] dark:text-[#71717a] dark:hover:text-[#f4f4f5] dark:hover:bg-[#17171b] active:bg-[#e4e4e7] dark:active:bg-[#232328] transition-colors cursor-pointer select-none"
        aria-label={t('window.minimize')}
      >
        <Minus className="h-3.5 w-3.5 block shrink-0" strokeWidth={1.75} />
      </button>

      {/* Maximize / Restore Button */}
      <button
        type="button"
        data-no-drag
        onMouseDown={(e) => e.stopPropagation()}
        onClick={() => void toggleMaximizeWindow().then(setIsMaximized)}
        className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:text-[#18181b] hover:bg-[#f4f4f5] dark:text-[#71717a] dark:hover:text-[#f4f4f5] dark:hover:bg-[#17171b] active:bg-[#e4e4e7] dark:active:bg-[#232328] transition-colors cursor-pointer select-none"
        aria-label={isMaximized ? t('window.restore') : t('window.maximize')}
      >
        {isMaximized ? (
          <Copy className="h-3 w-3 block shrink-0 -scale-y-100" strokeWidth={1.75} />
        ) : (
          <Square className="h-3 w-3 block shrink-0" strokeWidth={1.75} />
        )}
      </button>

      {/* Close Button: Signature Red on Hover */}
      <button
        type="button"
        data-no-drag
        onMouseDown={(e) => e.stopPropagation()}
        onClick={() => void closeWindow()}
        className="h-10 w-11 inline-flex items-center justify-center text-[#71717a] hover:bg-rose-600 hover:text-white active:bg-rose-700 transition-colors cursor-pointer select-none"
        aria-label={t('window.close')}
      >
        <X className="h-3.5 w-3.5 block shrink-0" strokeWidth={2} />
      </button>
    </div>
  );
}
