import React from 'react';
import {
  PythonIcon,
  TypeScriptIcon,
  JavaScriptIcon,
  RustIcon,
  ReactIcon,
  GoIcon,
  CppIcon,
  JsonIcon,
  HtmlIcon,
  CssIcon,
  MarkdownIcon,
  CSharpIcon,
  ShellIcon,
  ConfigIcon,
  SqlIcon,
} from './brandSvgIcons';

interface LangBadgeConfig {
  label: string;
  bg: string;
  text: string;
  border: string;
  icon: React.ReactNode;
}

export function getLanguageBadgeConfig(ext?: string): LangBadgeConfig | null {
  if (!ext) return null;
  switch (ext.toLowerCase()) {
    case 'py':
      return {
        label: 'PY',
        bg: 'bg-[#f0f7ff] dark:bg-[#0c2238]',
        text: 'text-[#1e6ca8] dark:text-[#58a6ff]',
        border: 'border-[#b9daf8] dark:border-[#1d4b73]',
        icon: <PythonIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'ts':
      return {
        label: 'TS',
        bg: 'bg-[#eff6ff] dark:bg-[#10243e]',
        text: 'text-[#2563eb] dark:text-[#60a5fa]',
        border: 'border-[#bfdbfe] dark:border-[#1d4ed8]',
        icon: <TypeScriptIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'tsx':
      return {
        label: 'TSX',
        bg: 'bg-[#ecfeff] dark:bg-[#082f38]',
        text: 'text-[#0891b2] dark:text-[#22d3ee]',
        border: 'border-[#a5f3fc] dark:border-[#0e7490]',
        icon: <ReactIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'js':
      return {
        label: 'JS',
        bg: 'bg-[#fefce8] dark:bg-[#2b2408]',
        text: 'text-[#a16207] dark:text-[#facc15]',
        border: 'border-[#fef08a] dark:border-[#854d0e]',
        icon: <JavaScriptIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'jsx':
      return {
        label: 'JSX',
        bg: 'bg-[#ecfeff] dark:bg-[#082f38]',
        text: 'text-[#0891b2] dark:text-[#22d3ee]',
        border: 'border-[#a5f3fc] dark:border-[#0e7490]',
        icon: <ReactIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'rs':
      return {
        label: 'RS',
        bg: 'bg-[#fff7ed] dark:bg-[#2d120a]',
        text: 'text-[#c2410c] dark:text-[#fb923c]',
        border: 'border-[#fed7aa] dark:border-[#7c2d12]',
        icon: <RustIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'go':
      return {
        label: 'GO',
        bg: 'bg-[#ecfeff] dark:bg-[#082a36]',
        text: 'text-[#007d9c] dark:text-[#38bdf8]',
        border: 'border-[#a5f3fc] dark:border-[#075985]',
        icon: <GoIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'cpp':
    case 'c':
    case 'h':
    case 'hpp':
      return {
        label: ext.toLowerCase().startsWith('c') ? 'C++' : 'H',
        bg: 'bg-[#eff6ff] dark:bg-[#142340]',
        text: 'text-[#00599c] dark:text-[#60a5fa]',
        border: 'border-[#bfdbfe] dark:border-[#1e40af]',
        icon: <CppIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'json':
      return {
        label: 'JSON',
        bg: 'bg-[#fefce8] dark:bg-[#25200c]',
        text: 'text-[#b45309] dark:text-[#fde047]',
        border: 'border-[#fef08a] dark:border-[#713f12]',
        icon: <JsonIcon className="h-3.5 w-3.5 shrink-0 text-[#b45309] dark:text-[#fde047]" />,
      };
    case 'html':
      return {
        label: 'HTML',
        bg: 'bg-[#fff1f2] dark:bg-[#2d0d15]',
        text: 'text-[#e11d48] dark:text-[#fb7185]',
        border: 'border-[#fecdd3] dark:border-[#9f1239]',
        icon: <HtmlIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'css':
    case 'scss':
      return {
        label: ext.toUpperCase(),
        bg: 'bg-[#f0f9ff] dark:bg-[#0c2338]',
        text: 'text-[#0284c7] dark:text-[#38bdf8]',
        border: 'border-[#bae6fd] dark:border-[#0369a1]',
        icon: <CssIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'md':
    case 'markdown':
      return {
        label: 'MD',
        bg: 'bg-[#f3f4f6] dark:bg-[#181a20]',
        text: 'text-[#374151] dark:text-[#d1d5db]',
        border: 'border-[#e5e7eb] dark:border-[#374151]',
        icon: <MarkdownIcon className="h-3.5 w-3.5 shrink-0 text-[#374151] dark:text-[#d1d5db]" />,
      };
    case 'cs':
      return {
        label: 'C#',
        bg: 'bg-[#ecfeff] dark:bg-[#082f38]',
        text: 'text-[#0e7490] dark:text-[#22d3ee]',
        border: 'border-[#a5f3fc] dark:border-[#155e75]',
        icon: <CSharpIcon className="h-3.5 w-3.5 shrink-0" />,
      };
    case 'sh':
    case 'bash':
    case 'ps1':
    case 'bat':
    case 'cmd':
      return {
        label: ext.toUpperCase() === 'BASH' ? 'SH' : ext.toUpperCase(),
        bg: 'bg-[#f0fdf4] dark:bg-[#0d2218]',
        text: 'text-[#047857] dark:text-[#34d399]',
        border: 'border-[#bbf7d0] dark:border-[#065f46]',
        icon: <ShellIcon className="h-3.5 w-3.5 shrink-0 text-[#047857] dark:text-[#34d399]" />,
      };
    case 'toml':
    case 'yaml':
    case 'yml':
      return {
        label: ext.toUpperCase(),
        bg: 'bg-[#f4f4f5] dark:bg-[#18181b]',
        text: 'text-[#52525b] dark:text-[#a1a1aa]',
        border: 'border-[#e4e4e7] dark:border-[#27272a]',
        icon: <ConfigIcon className="h-3.5 w-3.5 shrink-0 text-[#52525b] dark:text-[#a1a1aa]" />,
      };
    case 'sql':
      return {
        label: 'SQL',
        bg: 'bg-[#f0f9ff] dark:bg-[#0c2238]',
        text: 'text-[#0369a1] dark:text-[#38bdf8]',
        border: 'border-[#bae6fd] dark:border-[#075985]',
        icon: <SqlIcon className="h-3.5 w-3.5 shrink-0 text-[#0369a1] dark:text-[#38bdf8]" />,
      };
    default:
      return null;
  }
}

interface FileExtensionBadgeProps {
  ext?: string;
  className?: string;
}

export function FileExtensionBadge({ ext, className = '' }: FileExtensionBadgeProps) {
  const badge = getLanguageBadgeConfig(ext);
  if (!badge) return null;

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-mono font-semibold uppercase tracking-wider shrink-0 select-none leading-none ${badge.bg} ${badge.text} border ${badge.border} ${className}`}
    >
      {badge.icon}
      <span className="leading-none">{badge.label}</span>
    </span>
  );
}
