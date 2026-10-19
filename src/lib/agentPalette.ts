/**
 * Цветовая палитра для агентов (персон).
 * 
 * ВАЖНЫЙ ИНВАРИАНТ:
 * Серый (#64748b / #6b7280) и зеленый (#10b981 / #22c55e) цвета
 * НАВСЕГДА ЗАРЕЗЕРВИРОВАНЫ за системными колонками 'backlog' и 'ready'
 * и строго исключены из палитры выбора цветов агентов.
 */

export const BACKLOG_COLOR = '#64748b';
export const READY_COLOR = '#10b981';

export const FORBIDDEN_AGENT_COLORS = new Set([
  '#64748b',
  '#6b7280',
  '#10b981',
  '#22c55e',
]);

export const AGENT_PALETTE: readonly string[] = [
  '#f59e0b', // amber
  '#a855f7', // purple
  '#0284c7', // sky
  '#ef4444', // red
  '#6366f1', // indigo
  '#14b8a6', // teal
  '#ec4899', // pink
  '#f97316', // orange
  '#06b6d4', // cyan
  '#8b5cf6', // violet
];

export const AGENT_ICONS: readonly string[] = [
  'Bot',
  'Brain',
  'Cpu',
  'Code2',
  'ShieldCheck',
  'History',
  'Terminal',
  'Zap',
  'Flame',
  'Radar',
  'Workflow',
  'Boxes',
  'Wrench',
  'Rocket',
  'Fingerprint',
  'Compass',
  'Crosshair',
  'Layers',
];

/**
 * Детерминированный выбор цвета для роли агента из AGENT_PALETTE.
 */
export function getDeterministicAgentColor(role: string): string {
  const trimmed = role.trim();
  if (trimmed === 'cvh-dev') return '#6366f1';
  if (trimmed === 'developer' || trimmed === 'dev') return '#0284c7';
  if (trimmed === 'reviewer' || trimmed === 'review') return '#a855f7';
  if (trimmed === 'retro') return '#f59e0b';
  
  let hash = 5381;
  for (let i = 0; i < trimmed.length; i++) {
    hash = (hash * 33 + trimmed.charCodeAt(i)) >>> 0;
  }
  return AGENT_PALETTE[hash % AGENT_PALETTE.length];
}

/**
 * Проверка, является ли цвет допустимым для агента (не серый и не зеленый).
 */
export function isAllowedAgentColor(color?: string | null): boolean {
  if (!color) return false;
  const normalized = color.trim().toLowerCase();
  return !FORBIDDEN_AGENT_COLORS.has(normalized);
}
