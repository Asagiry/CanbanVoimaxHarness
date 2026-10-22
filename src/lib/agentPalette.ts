import type { CSSProperties } from 'react';

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

const FORBIDDEN_AGENT_COLORS = new Set([
  '#64748b',
  '#6b7280',
  '#10b981',
  '#22c55e',
]);

export const AGENT_PALETTE: readonly string[] = [
  '#f59e0b', // amber
  '#a855f7', // purple
  '#0284c7', // sky
  '#3b82f6', // blue (pleasant non-aggressive blue, replacing red alert)
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
  'Sparkles',
  'History',
  'Terminal',
  'Zap',
  'Flame',
  'Radar',
  'Workflow',
  'Boxes',
  'Wrench',
  'Rocket',
  'Compass',
  'Crosshair',
  'Layers',
];

/**
 * Детерминированный выбор цвета для роли агента из AGENT_PALETTE.
 */
export function getDeterministicAgentColor(role: string): string {
  if (!role) return '#6366f1';
  const trimmed = role.trim();
  if (!trimmed) return '#6366f1';
  const lower = trimmed.toLowerCase();
  if (lower === 'cvh-dev') return '#6366f1';
  if (lower === 'developer' || lower === 'dev') return '#0284c7';
  if (lower === 'reviewer' || lower === 'review') return '#a855f7';
  if (lower === 'retro') return '#f59e0b';
  if (lower === 'general') return '#6366f1';
  if (lower === 'oneshot') return '#0284c7';
  
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

/**
 * Детерминированный выбор иконки для роли агента из AGENT_ICONS.
 */
export function getDeterministicAgentIcon(role?: string | null): string {
  if (!role) return 'Bot';
  const trimmed = role.trim();
  if (!trimmed) return 'Bot';
  const lower = trimmed.toLowerCase();
  if (lower === 'cvh-dev') return 'Cpu';
  if (lower === 'developer' || lower === 'dev') return 'Code2';
  if (lower === 'reviewer' || lower === 'review') return 'ShieldCheck';
  if (lower === 'retro') return 'History';
  if (lower === 'oneshot') return 'Zap';
  if (lower === 'general') return 'Sparkles';

  let hash = 5381;
  for (let i = 0; i < trimmed.length; i++) {
    hash = (hash * 33 + trimmed.charCodeAt(i)) >>> 0;
  }
  return AGENT_ICONS[hash % AGENT_ICONS.length] || 'Bot';
}

/**
 * Возвращает динамический inline-стиль для бейджа роли агента на основе его цвета.
 */
export function getRoleBadgeStyle(role?: string | null, customColor?: string | null): CSSProperties {
  const color = (customColor && isAllowedAgentColor(customColor))
    ? customColor
    : getDeterministicAgentColor(role || 'general');
  return {
    color,
    borderColor: `${color}50`,
    backgroundColor: `${color}14`,
  };
}

/**
 * Возвращает динамический inline-стиль для активной карточки субагента в сайдбаре.
 */
export function getRoleActiveCardStyle(role?: string | null, customColor?: string | null): CSSProperties {
  const color = (customColor && isAllowedAgentColor(customColor))
    ? customColor
    : getDeterministicAgentColor(role || 'general');
  return {
    borderColor: `${color}99`,
    backgroundColor: `${color}12`,
    boxShadow: `0 0 0 1px ${color}33`,
  };
}
