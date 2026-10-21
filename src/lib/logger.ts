/**
 * End-to-end file logging utility for CanbanVoimaxHarness.
 * Forwards log events to backend Tauri command `log_event` which writes to `.cvh/logs/cvh.log`
 * with timestamp, level, tag, and optional details.
 */
export async function logEvent(
  level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG',
  tag: string,
  message: string,
  details?: string
): Promise<void> {
  const ts = new Date().toISOString();
  const consoleFn = level === 'ERROR' ? console.error : level === 'WARN' ? console.warn : console.log;
  consoleFn(`[${ts}] [${level}] [${tag}] ${message}`, details ? `| details: ${details}` : '');
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('log_event', {
      level,
      tag,
      message,
      details: details || null,
    });
  } catch (err) {
    // Graceful fallback for non-Tauri / web environments
    console.debug('[logEvent] Tauri invoke not available or failed:', err);
  }
}
