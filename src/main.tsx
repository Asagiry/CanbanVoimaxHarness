import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';
import { useLanguageStore } from './stores/languageStore';

// Sync <html lang> dynamically with user language preference
if (typeof document !== 'undefined') {
  document.documentElement.lang = useLanguageStore.getState().language;
  useLanguageStore.subscribe((state) => {
    document.documentElement.lang = state.language;
  });
}

// Forward renderer crashes to the backend log so white-screens are diagnosable.
// Fire-and-forget, throttled, never throws (must not create error loops).
let uiErrorCount = 0;
function reportUiError(kind: string, message: string, stack?: string): void {
  if (uiErrorCount >= 20) return;
  uiErrorCount += 1;
  try {
    const text = `[${kind}] ${message}${stack ? `\nSTACK: ${stack}` : ''}`.slice(0, 4000);
    void import('@tauri-apps/api/core')
      .then(({ invoke }) =>
        invoke<void>('log_ui_error', { message: text }).catch((err: unknown) => {
          console.debug('[reportUiError] log_ui_error failed:', err);
        }),
      )
      .catch((err: unknown) => {
        console.debug('[reportUiError] import failed:', err);
      });
  } catch (err) {
    console.debug('[reportUiError] unexpected error:', err);
  }
}

window.addEventListener('error', (e) => {
  const stack = e.error instanceof Error ? e.error.stack : undefined;
  reportUiError('window.onerror', e.message, stack);
});

window.addEventListener('unhandledrejection', (e: PromiseRejectionEvent) => {
  const reason: unknown = e.reason;
  const stack = reason instanceof Error ? reason.stack : undefined;
  reportUiError(
    'unhandledrejection',
    typeof reason === 'string' ? reason : reason instanceof Error ? reason.message : String(reason),
    stack,
  );
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
