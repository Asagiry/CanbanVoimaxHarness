/**
 * Default icon resolver for Kanban columns based on their canonical IDs.
 */
export function getDefaultColumnIcon(columnId: string, fallback?: string): string {
  switch (columnId) {
    case 'backlog':
      return 'Inbox';
    case 'dev':
      return 'Code2';
    case 'review':
      return 'ShieldCheck';
    case 'retro':
      return 'History';
    case 'ready':
      return 'CheckCircle2';
    default:
      return fallback || 'Layers';
  }
}
