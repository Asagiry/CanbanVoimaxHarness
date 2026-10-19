import {
  Bot,
  Brain,
  Cpu,
  Code2,
  ShieldCheck,
  Sparkles,
  Terminal,
  Zap,
  Flame,
  Radar,
  Workflow,
  Boxes,
  Wrench,
  Rocket,
  Fingerprint,
  Compass,
  Crosshair,
  Layers,
  History,
  Inbox,
  ListTodo,
  CheckCircle2,
  User,
  type LucideProps,
} from 'lucide-react';
import type { ComponentType } from 'react';

export const CANONICAL_AGENT_ICONS = [
  'Bot',
  'Brain',
  'Cpu',
  'Code2',
  'ShieldCheck',
  'Sparkles',
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
  'History',
] as const;

export const AGENT_ICON_MAP: Record<string, ComponentType<LucideProps>> = {
  Bot,
  Brain,
  Cpu,
  Code2,
  ShieldCheck,
  Sparkles,
  Terminal,
  Zap,
  Flame,
  Radar,
  Workflow,
  Boxes,
  Wrench,
  Rocket,
  Fingerprint,
  Compass,
  Crosshair,
  Layers,
  History,
  Inbox,
  ListTodo,
  CheckCircle2,
  User,
};

export interface AgentIconProps extends Omit<LucideProps, 'name'> {
  name?: string | null;
  className?: string;
}

export default function AgentIcon({ name, className = 'h-3.5 w-3.5 shrink-0', ...props }: AgentIconProps) {
  const IconComponent = (name && AGENT_ICON_MAP[name]) || Layers;
  return <IconComponent className={className} {...props} />;
}
