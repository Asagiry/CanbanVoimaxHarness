import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';

// Forward renderer crashes to the backend log so white-screens are diagnosable.
// Fire-and-forget, throttled, never throws (must not create error loops).
let uiErrorCount = 0;
function reportUiError(kind: string, message: string, stack?: string) {
  if (uiErrorCount >= 20) return;
  uiErrorCount += 1;
  try {
    const text = `[${kind}] ${message}${stack ? `\nSTACK: ${stack}` : ''}`.slice(0, 4000);
    import('@tauri-apps/api/core')
      .then(({ invoke }) => {
        (invoke as any)('log_ui_error', { message: text }).catch(() => {});
      })
      .catch(() => {});
  } catch {
    // ignore
  }
}
window.addEventListener('error', (e) => {
  reportUiError('window.onerror', e.message, (e.error as any)?.stack);
});
window.addEventListener('unhandledrejection', (e) => {
  const reason = (e as PromiseRejectionEvent).reason;
  reportUiError(
    'unhandledrejection',
    typeof reason === 'string' ? reason : String(reason),
    (reason as any)?.stack
  );
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
