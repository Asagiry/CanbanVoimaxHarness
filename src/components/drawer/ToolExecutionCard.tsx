import { useState } from 'react';
import { ToolExecution } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import {
  ChevronRight,
  Copy,
  Check,
  X,
  Wrench,
  FileCode2,
  FileText,
} from 'lucide-react';

interface ToolExecutionCardProps {
  tool: ToolExecution;
}

interface LangBadgeConfig {
  label: string;
  bg: string;
  text: string;
  border: string;
}

function getLanguageBadge(ext?: string): LangBadgeConfig | null {
  if (!ext) return null;
  switch (ext.toLowerCase()) {
    case 'cs':
      return { label: 'C#', bg: 'bg-[#ecfeff]', text: 'text-[#0891b2]', border: 'border-[#a5f3fc]' };
    case 'ts':
    case 'tsx':
      return { label: 'TS', bg: 'bg-[#eff6ff]', text: 'text-[#2563eb]', border: 'border-[#bfdbfe]' };
    case 'js':
    case 'jsx':
      return { label: 'JS', bg: 'bg-[#fefce8]', text: 'text-[#ca8a04]', border: 'border-[#fef08a]' };
    case 'py':
      return { label: 'PY', bg: 'bg-[#f0f9ff]', text: 'text-[#0284c7]', border: 'border-[#bae6fd]' };
    case 'rs':
      return { label: 'RS', bg: 'bg-[#fff7ed]', text: 'text-[#ea580c]', border: 'border-[#fed7aa]' };
    case 'json':
      return { label: 'JSON', bg: 'bg-[#f4f4f5]', text: 'text-[#52525b]', border: 'border-[#e4e4e7]' };
    case 'html':
      return { label: 'HTML', bg: 'bg-[#fff1f2]', text: 'text-[#e11d48]', border: 'border-[#fecdd3]' };
    case 'css':
    case 'scss':
      return { label: 'CSS', bg: 'bg-[#f0fdf4]', text: 'text-[#16a34a]', border: 'border-[#bbf7d0]' };
    case 'go':
      return { label: 'GO', bg: 'bg-[#ecfeff]', text: 'text-[#06b6d4]', border: 'border-[#a5f3fc]' };
    case 'cpp':
    case 'c':
    case 'h':
    case 'hpp':
      return { label: 'C++', bg: 'bg-[#fdf2f8]', text: 'text-[#db2777]', border: 'border-[#fbcfe8]' };
    case 'sh':
    case 'bash':
    case 'ps1':
    case 'bat':
    case 'cmd':
      return { label: 'SH', bg: 'bg-[#f8fafc]', text: 'text-[#475569]', border: 'border-[#cbd5e1]' };
    case 'md':
      return { label: 'MD', bg: 'bg-[#fafafa]', text: 'text-[#71717a]', border: 'border-[#e4e4e7]' };
    case 'toml':
    case 'yaml':
    case 'yml':
      return { label: 'CFG', bg: 'bg-[#f4f4f5]', text: 'text-[#52525b]', border: 'border-[#e4e4e7]' };
    case 'sql':
      return { label: 'SQL', bg: 'bg-[#faf5ff]', text: 'text-[#9333ea]', border: 'border-[#e9d5ff]' };
    default:
      return null;
  }
}

export function getRoleBadgeClass(role?: string | null): string {
  const roleUpper = (role || 'general').toUpperCase();
  if (roleUpper.includes('DEV')) {
    return 'bg-[#f0fdf4] text-[#15803d] border-[#bbf7d0]';
  }
  if (roleUpper.includes('REV')) {
    return 'bg-[#fefce8] text-[#854d0e] border-[#fef08a]';
  }
  if (roleUpper.includes('EXP') || roleUpper.includes('ARCH')) {
    return 'bg-[#faf5ff] text-[#7c3aed] border-[#e9d5ff]';
  }
  return 'bg-[#f4f4f5] text-[#171717] border-[#ebebeb]';
}

export default function ToolExecutionCard({ tool }: ToolExecutionCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  if (!tool) return null;

  const toolName: string = (tool as ToolExecution)?.toolName ?? 'tool';
  const params: Record<string, unknown> =
    (tool as ToolExecution)?.params && typeof (tool as ToolExecution).params === 'object'
      ? ((tool as ToolExecution).params as Record<string, unknown>)
      : {};

  const openSubagent = useDrawerStore((s) => s.openSubagent);

  // Subagent interactive card (OpenCode style per specifications)
  if (toolName === 'subagent' || toolName === 'subagent_bg') {
    const rawRole = (params?.role as string) || 'general';
    const roleUpper = rawRole.toUpperCase();
    const title = (params?.title as string) || (params?.prompt as string)?.slice(0, 60) || 'Делегированная задача субагента';
    const prompt = (params?.prompt as string) || '';

    // Extract subagent_id from params, or drawerStore toolIdToSubagentId map, or parsed JSON stdout (subagent_bg), or stdout regex (subagent)
    let subagentId = typeof params?.subagent_id === 'string' && params.subagent_id ? params.subagent_id : '';
    if (!subagentId) {
      const mapped = useDrawerStore.getState().toolIdToSubagentId[tool.id];
      if (mapped) subagentId = mapped;
    }
    if (!subagentId && tool.stdout) {
      try {
        const parsed = JSON.parse(tool.stdout);
        if (parsed.subagent_id) subagentId = parsed.subagent_id;
      } catch {}
      if (!subagentId) {
        const match = tool.stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
        if (match && match[1]) subagentId = match[1];
      }
    }
    if (!subagentId) {
      subagentId = tool.id;
    }

    const roleBadgeClass = getRoleBadgeClass(rawRole);

    const isRunning = tool.status === 'running';
    const isSuccess = tool.status === 'success';

    return (
      <div
        onClick={() => openSubagent(subagentId, title, rawRole, prompt)}
        className="group rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] hover:bg-[#f9f9f9] dark:hover:bg-[#1c2128] hover:border-[#171717]/40 dark:hover:border-[#30363d] p-3.5 shadow-xs transition-all duration-150 cursor-pointer select-none my-2.5"
        title="Нажмите, чтобы открыть вложенный чат субагента"
      >
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center space-x-2.5 min-w-0 flex-1 overflow-hidden">
            <span className="text-[#171717] dark:text-[#f0f6fc] shrink-0 font-mono text-sm leading-none select-none">
              ■
            </span>

            <span className={`px-2 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider border shrink-0 select-none ${roleBadgeClass}`}>
              {roleUpper}
            </span>

            <span className="font-sans font-medium text-xs md:text-sm text-[#0d0d0d] dark:text-[#f0f6fc] group-hover:text-[#171717] dark:group-hover:text-white transition-colors truncate">
              {title}
            </span>
          </div>

          <div className="flex items-center space-x-2.5 shrink-0">
            {isRunning ? (
              <div className="flex items-center space-x-1.5">
                <span className="relative flex h-2 w-2 shrink-0" title="Субагент выполняется...">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-amber-500" />
                </span>
                <span className="text-[11px] font-mono text-[#71717a]">Выполняется</span>
              </div>
            ) : isSuccess ? (
              <div className="flex items-center space-x-1.5">
                <Check className="h-3.5 w-3.5 text-[#15803d] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#15803d]">Готово</span>
              </div>
            ) : (
              <div className="flex items-center space-x-1.5">
                <X className="h-3.5 w-3.5 text-[#dc2626] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#dc2626]">Ошибка</span>
              </div>
            )}

            <ChevronRight className="h-4 w-4 text-[#8f8f8f] group-hover:text-[#171717] group-hover:translate-x-0.5 transition-all shrink-0" />
          </div>
        </div>
      </div>
    );
  }

  // Check if tool is a file modification/creation operation
  const isFileOp =
    toolName === 'write_file' ||
    toolName === 'write' ||
    toolName === 'edit_file' ||
    toolName === 'edit' ||
    toolName === 'str_replace';

  // Check if tool is a file read operation
  const isReadOp =
    toolName === 'read_file' ||
    toolName === 'read' ||
    toolName === 'fetch_file';

  const isEditTool =
    toolName === 'edit_file' ||
    toolName === 'str_replace' ||
    toolName === 'edit';
  const isDiffTool = isEditTool;

  const rawFilePath =
    (params?.file_path as string) ||
    (params?.path as string) ||
    (params?.filePath as string) ||
    (params?.target_file as string) ||
    '';

  const normalizedPath = rawFilePath.replace(/\\/g, '/');
  const fileName = normalizedPath.split('/').pop() || rawFilePath;
  const dirPath = normalizedPath.substring(0, normalizedPath.length - fileName.length).replace(/\/$/, '');

  const oldString =
    (params?.old_string as string | undefined) ??
    (params?.old_str as string | undefined) ??
    (params?.before as string | undefined);

  const newString =
    (params?.new_string as string | undefined) ??
    (params?.new_str as string | undefined) ??
    (params?.content as string | undefined);

  // Compute delta additions and deletions (+X -Y)
  let additions = 0;
  let deletions = 0;

  if (oldString !== undefined && oldString !== '') {
    deletions = oldString.split('\n').length;
  }
  if (newString !== undefined && newString !== '') {
    additions = newString.split('\n').length;
  }

  // Fallback for line counts if not computed from strings
  if (additions === 0 && deletions === 0) {
    if (typeof params?.lines === 'number') {
      additions = params.lines;
    } else if (tool.stdout) {
      const match = tool.stdout.match(/\+(\d+)\s*-(\d+)/);
      if (match) {
        additions = parseInt(match[1], 10);
        deletions = parseInt(match[2], 10);
      }
    }
  }

  const isFailed =
    tool.status === 'failed' ||
    (tool.status as string) === 'error' ||
    Boolean(tool.stderr && tool.stderr.trim().length > 0) ||
    (tool.exitCode !== undefined && tool.exitCode !== null && tool.exitCode !== 0);

  // Label: "Created" (if new file) or "Edited" (if edit/overwrite)
  let opLabel: 'Created' | 'Edited' = 'Edited';
  if (isEditTool) {
    opLabel = 'Edited';
  } else {
    const op = (params?.operation as string)?.toLowerCase();
    if (op === 'create' || params?.is_new === true) {
      opLabel = 'Created';
    } else if (op === 'update' || deletions > 0 || params?.before !== undefined) {
      opLabel = 'Edited';
    } else {
      const stdoutLower = (tool.stdout ?? '').toLowerCase();
      if (stdoutLower.includes('created') || stdoutLower.includes('new file')) {
        opLabel = 'Created';
      } else if (stdoutLower.includes('update') || stdoutLower.includes('overwrit') || stdoutLower.includes('replaced')) {
        opLabel = 'Edited';
      } else {
        opLabel = deletions === 0 ? 'Created' : 'Edited';
      }
    }
  }

  // Select code icon vs generic text icon
  const ext = fileName.includes('.') ? fileName.split('.').pop()?.toLowerCase() : '';
  const isCode = ['ts', 'tsx', 'js', 'jsx', 'rs', 'py', 'json', 'toml', 'yaml', 'yml', 'xaml', 'xml', 'html', 'css', 'scss', 'sh', 'ps1', 'bat'].includes(ext || '');
  const DocIcon = isCode ? FileCode2 : FileText;
  const langBadge = getLanguageBadge(ext);

  // Line range calculations for read operations
  const offsetVal =
    params?.offset !== undefined && params?.offset !== null
      ? Number(params.offset)
      : undefined;
  const limitVal =
    params?.limit !== undefined && params?.limit !== null
      ? Number(params.limit)
      : undefined;
  const startLine =
    offsetVal !== undefined && !isNaN(offsetVal)
      ? Math.max(1, offsetVal)
      : limitVal !== undefined && !isNaN(limitVal)
      ? 1
      : undefined;
  const endLine =
    startLine !== undefined && limitVal !== undefined && !isNaN(limitVal)
      ? startLine + limitVal - 1
      : undefined;

  let lineRangeBadge: string | null = null;
  if (startLine !== undefined && endLine !== undefined) {
    lineRangeBadge = `#L${startLine}-${endLine}`;
  } else if (startLine !== undefined) {
    lineRangeBadge = `#L${startLine}+`;
  } else if (tool.stdout) {
    const stdoutLines = tool.stdout.split('\n').filter((l) => l.trim().length > 0).length;
    lineRangeBadge = `· ${stdoutLines} lines`;
  } else if (limitVal !== undefined) {
    lineRangeBadge = `· ${limitVal} lines`;
  }

  const getTarget = (): { target: string; fullTarget: string } => {
    const p = params ?? {};
    const str = (v: unknown): string | undefined =>
      typeof v === 'string' && v.trim().length > 0 ? v.trim() : undefined;

    let raw = '';
    switch (tool.toolName) {
      case 'write':
      case 'write_file':
      case 'read':
      case 'read_file':
      case 'edit':
      case 'edit_file':
      case 'str_replace':
        raw = str(p.file_path) || str(p.path) || '';
        break;
      case 'glob':
        raw = str(p.pattern) || '';
        break;
      case 'grep': {
        const pattern = str(p.pattern) || '';
        const path = str(p.path);
        raw = path && path !== '.' ? `${pattern} in ${path}` : pattern;
        break;
      }
      case 'execute_command':
      case 'bash':
      case 'pwsh':
        raw = str(p.command) || str(p.cmd) || '';
        break;
      case 'web_search':
        raw = str(p.query) || '';
        break;
      case 'web_fetch':
        raw = str(p.url) || '';
        break;
      case 'subagent':
        raw = str(p.role) || str(p.description) || str(p.prompt) || '';
        break;
      default: {
        const firstStr = Object.values(p).find(
          (v) => typeof v === 'string' && (v as string).trim().length > 0
        ) as string | undefined;
        raw = firstStr || '';
        break;
      }
    }

    const fullTarget = raw;
    const target = raw.length > 55 ? `${raw.slice(0, 52)}…` : raw;
    return { target, fullTarget };
  };

  const getExtraInfo = (): string | null => {
    const p = tool.params ?? {};
    switch (tool.toolName) {
      case 'read':
      case 'read_file': {
        if (tool.stdout) {
          const lines = tool.stdout.split('\n').length;
          if (lines > 1) return `${lines} lines`;
        }
        if (typeof p.limit === 'number') {
          return `${p.limit} lines`;
        }
        return null;
      }
      case 'glob': {
        if (tool.stdout) {
          const count = tool.stdout
            .split('\n')
            .filter((l) => l.trim().length > 0).length;
          return `${count} files`;
        }
        return null;
      }
      case 'grep': {
        if (tool.stdout) {
          const count = tool.stdout
            .split('\n')
            .filter((l) => l.trim().length > 0).length;
          return `${count} matches`;
        }
        return null;
      }
      case 'execute_command':
      case 'bash':
      case 'pwsh': {
        if (tool.status === 'success') {
          return 'exit 0';
        }
        if (tool.status === 'failed') {
          return `exit ${tool.exitCode ?? 1}`;
        }
        return null;
      }
      default: {
        if (tool.stdout) {
          const lines = tool.stdout.split('\n').length;
          if (lines > 1) return `${lines} lines`;
        }
        return null;
      }
    }
  };

  const formatDuration = (ms: number): string | null => {
    if (!ms || ms <= 0) return null;
    if (ms >= 1000) {
      return `${(ms / 1000).toFixed(1)}s`;
    }
    return `${ms}ms`;
  };

  const { target, fullTarget } = getTarget();
  const extraInfo = getExtraInfo();
  const durationText = formatDuration(tool.durationMs);

  const handleCopyLogs = (e: React.MouseEvent) => {
    e.stopPropagation();
    const formattedParams =
      tool.params && Object.keys(tool.params).length > 0
        ? Object.entries(tool.params as Record<string, unknown>)
            .map(([k, v]) => {
              if (typeof v === 'string') {
                return v.includes('\n') ? `${k}:\n${v}` : `${k}: ${v}`;
              }
              return `${k}: ${JSON.stringify(v, null, 2)}`;
            })
            .join('\n\n')
        : '';

    const content = [
      `Инструмент: ${tool.toolName}`,
      `Статус: ${tool.status} (код ${tool.exitCode ?? 0})`,
      formattedParams ? `ПАРАМЕТРЫ:\n${formattedParams}` : '',
      tool.stdout ? `STDOUT:\n${tool.stdout}` : '',
      tool.stderr ? `STDERR:\n${tool.stderr}` : '',
    ]
      .filter(Boolean)
      .join('\n\n');

    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const renderStatusIndicator = () => {
    if (tool.status === 'running') {
      return (
        <span
          className="relative flex h-2 w-2 shrink-0"
          title="Выполняется..."
        >
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75" />
          <span className="relative inline-flex rounded-full h-2 w-2 bg-amber-500" />
        </span>
      );
    }
    if (tool.status === 'success') {
      return (
        <span title={`Успешно (код ${tool.exitCode ?? 0})`} className="flex items-center shrink-0">
          <Check className="h-3.5 w-3.5 text-emerald-600 stroke-[2.5]" />
        </span>
      );
    }
    return (
      <span title={`Ошибка (код ${tool.exitCode ?? 1})`} className="flex items-center shrink-0">
        <X className="h-3.5 w-3.5 text-rose-600 stroke-[2.5]" />
      </span>
    );
  };

  const renderUnifiedDiff = () => {
    if (oldString !== undefined || newString !== undefined) {
      const oldLines = oldString ? oldString.split('\n') : [];
      const newLines = newString ? newString.split('\n') : [];

      return (
        <div className="rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[11px]">
          {rawFilePath && (
            <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] border-b border-[#ebebeb] text-[#666666] text-[10px]">
              <span className="truncate max-w-[340px] text-[#171717] font-semibold" title={rawFilePath}>
                {rawFilePath}
              </span>
              <div className="flex items-center space-x-2 font-mono tabular-nums">
                {oldLines.length > 0 && (
                  <span className="text-[#dc2626]">-{oldLines.length}</span>
                )}
                {newLines.length > 0 && (
                  <span className="text-[#16a34a]">+{newLines.length}</span>
                )}
                <span className="text-[#8f8f8f]">Различия</span>
              </div>
            </div>
          )}

          <div className="divide-y divide-[#ebebeb] select-text overflow-x-auto max-h-72 font-mono">
            {oldLines.map((line, idx) => (
              <div
                key={`old-${idx}`}
                className="flex items-start bg-[#fef2f2] text-[#991b1b] px-2 py-0.5 leading-relaxed font-mono"
              >
                <span className="w-4 shrink-0 text-[#ef4444] select-none text-center font-bold">
                  -
                </span>
                <span className="w-8 shrink-0 text-[#8f8f8f] select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">
                  {line || ' '}
                </span>
              </div>
            ))}

            {newLines.map((line, idx) => (
              <div
                key={`new-${idx}`}
                className="flex items-start bg-[#f0fdf4] text-[#166534] px-2 py-0.5 leading-relaxed font-mono"
              >
                <span className="w-4 shrink-0 text-[#22c55e] select-none text-center font-bold">
                  +
                </span>
                <span className="w-8 shrink-0 text-[#8f8f8f] select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">
                  {line || ' '}
                </span>
              </div>
            ))}
          </div>
        </div>
      );
    }

    return null;
  };

  const renderFileContentPreview = () => {
    if (!newString) return null;
    const lines = newString.split('\n');

    return (
      <div className="rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[11px]">
        {rawFilePath && (
          <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] border-b border-[#ebebeb] text-[#666666] text-[10px]">
            <span className="truncate max-w-[340px] text-[#171717] font-semibold" title={rawFilePath}>
              {rawFilePath}
            </span>
            <span className="text-[#8f8f8f] tabular-nums font-mono">
              {lines.length} {lines.length === 1 ? 'строка' : 'строк'}
            </span>
          </div>
        )}
        <div className="divide-y divide-[#f0f0f0] select-text overflow-x-auto max-h-72 font-mono bg-white">
          {lines.map((line, idx) => (
            <div
              key={`line-${idx}`}
              className="flex items-start px-2 py-0.5 leading-relaxed hover:bg-[#fafafa]"
            >
              <span className="w-8 shrink-0 text-[#a1a1aa] select-none text-right pr-2.5 tabular-nums text-[10px]">
                {idx + 1}
              </span>
              <span className="whitespace-pre-wrap break-all text-[#171717]">
                {line || ' '}
              </span>
            </div>
          ))}
        </div>
      </div>
    );
  };

  const renderReadContent = () => {
    if (!tool.stdout) return null;
    const rawLines = tool.stdout.split('\n');
    if (rawLines.length > 0 && rawLines[rawLines.length - 1] === '') {
      rawLines.pop();
    }
    if (rawLines.length === 0) return null;

    const baseLineNumber = startLine ?? 1;

    return (
      <div className="rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[11px]">
        {rawFilePath && (
          <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] border-b border-[#ebebeb] text-[#666666] text-[10px]">
            <span className="truncate max-w-[340px] text-[#171717] font-semibold" title={rawFilePath}>
              {rawFilePath}
            </span>
            <span className="text-[#8f8f8f] tabular-nums font-mono">
              {lineRangeBadge || `${rawLines.length} строк`}
            </span>
          </div>
        )}
        <div className="divide-y divide-[#f0f0f0] select-text overflow-x-auto max-h-80 font-mono bg-white">
          {rawLines.map((line, idx) => {
            const prefixMatch = line.match(/^(\d+)[:|]\s?(.*)$/);
            let displayLineNum: number | string = baseLineNumber + idx;
            let displayContent = line;

            if (prefixMatch) {
              displayLineNum = parseInt(prefixMatch[1], 10);
              displayContent = prefixMatch[2];
            }

            return (
              <div
                key={`readline-${idx}`}
                className="flex items-start px-2 py-0.5 leading-relaxed hover:bg-[#fafafa]"
              >
                <span className="w-10 shrink-0 text-[#a1a1aa] select-none text-right pr-2.5 tabular-nums text-[10px]">
                  {displayLineNum}
                </span>
                <span className="whitespace-pre-wrap break-all text-[#171717]">
                  {displayContent || ' '}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  const renderToolParams = (paramsObj: Record<string, unknown>) => {
    const entries = Object.entries(paramsObj);
    if (entries.length === 0) return null;

    return (
      <div className="space-y-1.5">
        <span className="text-[10px] text-[#666666] block font-mono">
          Параметры вызова
        </span>
        <div className="space-y-2">
          {entries.map(([key, value]) => {
            if (typeof value === 'string') {
              const isMultiline = value.includes('\n');
              const isCodeLike =
                key === 'content' ||
                key === 'code' ||
                key === 'command' ||
                key === 'cmd' ||
                key === 'old_string' ||
                key === 'new_string' ||
                key === 'prompt' ||
                key === 'script' ||
                key === 'patch' ||
                isMultiline;

              if (isCodeLike || isMultiline) {
                const lineCount = value.split('\n').length;
                return (
                  <div key={key} className="space-y-1">
                    <div className="flex items-center justify-between text-[10px] text-[#666666]">
                      <span className="font-semibold text-[#171717] font-mono">{key}:</span>
                      <span className="text-[#8f8f8f] tabular-nums font-mono text-[10px]">
                        {lineCount > 1 ? `${lineCount} строк` : `${value.length} симв.`}
                      </span>
                    </div>
                    <pre className="p-2.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[11px] text-[#171717] overflow-x-auto leading-relaxed max-h-64 whitespace-pre select-text font-mono">
                      {value}
                    </pre>
                  </div>
                );
              }

              return (
                <div key={key} className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] font-semibold shrink-0">{key}:</span>
                  <span className="text-[#171717] select-text break-all">{value}</span>
                </div>
              );
            }

            if (typeof value === 'object' && value !== null) {
              return (
                <div key={key} className="space-y-1">
                  <span className="text-[10px] font-semibold text-[#171717] font-mono">{key}:</span>
                  <pre className="p-2 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[11px] text-[#171717] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text font-mono">
                    {JSON.stringify(value, null, 2)}
                  </pre>
                </div>
              );
            }

            return (
              <div key={key} className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                <span className="text-[#666666] font-semibold shrink-0">{key}:</span>
                <span className="text-[#171717] select-text">{String(value)}</span>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <div className="group rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] hover:border-[#d4d4d8] dark:hover:border-[#30363d] overflow-hidden text-xs shadow-2xs transition-colors my-1">
      {/* Header Strip */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-2.5 py-1.5 hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] cursor-pointer transition select-none gap-2 text-xs min-h-[30px]"
      >
        {isReadOp ? (
          /* Read operation tactile card: [Analyzed C# Transit.cs #L4270-4300] */
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <button
              type="button"
              className="p-0.5 text-[#8f8f8f] hover:text-[#171717] rounded shrink-0 transition cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
              title={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
            >
              <ChevronRight
                className={`h-3 w-3 transition-transform duration-150 ${
                  isExpanded ? 'rotate-90 text-[#171717]' : ''
                }`}
              />
            </button>

            {/* Label: Analyzed (cyan/teal tactile badge) */}
            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold uppercase tracking-wider shrink-0 select-none bg-[#ecfeff] text-[#0e7490] border border-[#a5f3fc]">
              Analyzed
            </span>

            {/* Language badge or document icon */}
            {langBadge ? (
              <span
                className={`px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold uppercase tracking-wider shrink-0 select-none ${langBadge.bg} ${langBadge.text} border ${langBadge.border}`}
              >
                {langBadge.label}
              </span>
            ) : (
              <DocIcon className="h-3.5 w-3.5 text-[#52525b] shrink-0" />
            )}

            {/* Clean filename with muted path */}
            <div
              className="flex items-center min-w-0 font-mono text-xs overflow-hidden"
              title={rawFilePath}
            >
              {dirPath && (
                <span className="text-[#8f8f8f] truncate max-w-[120px] sm:max-w-[180px] shrink-0">
                  {dirPath}/
                </span>
              )}
              <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
                {fileName || rawFilePath || 'file'}
              </span>
            </div>

            {/* Line range badge (#L4270-4300 or · 61 lines) */}
            {lineRangeBadge && (
              <span className="font-mono text-[11px] text-[#71717a] tabular-nums shrink-0 ml-1">
                {lineRangeBadge}
              </span>
            )}

            {durationText && (
              <>
                <span className="text-[#a1a1aa] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] shrink-0 tabular-nums text-[11px]">
                  {durationText}
                </span>
              </>
            )}
          </div>
        ) : isFileOp ? (
          /* File operation card in screenshot style: [Edited 📄 MainWindow.xaml +4 -2] */
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <button
              type="button"
              className="p-0.5 text-[#8f8f8f] hover:text-[#171717] rounded shrink-0 transition cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
              title={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
            >
              <ChevronRight
                className={`h-3 w-3 transition-transform duration-150 ${
                  isExpanded ? 'rotate-90 text-[#171717]' : ''
                }`}
              />
            </button>

            {/* Label: Created or Edited or FAILED EDIT */}
            <span
              className={`px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold uppercase tracking-wider shrink-0 select-none ${
                isFailed
                  ? 'bg-[#fef2f2] text-[#b91c1c] border border-[#fecaca]'
                  : opLabel === 'Created'
                  ? 'bg-[#ecfdf5] text-[#047857] border border-[#a7f3d0]'
                  : 'bg-[#eff6ff] text-[#1d4ed8] border border-[#bfdbfe]'
              }`}
            >
              {isFailed ? 'FAILED EDIT' : opLabel}
            </span>

            {/* Document icon */}
            <DocIcon className="h-3.5 w-3.5 text-[#52525b] shrink-0" />

            {/* Clean filename with muted path */}
            <div
              className="flex items-center min-w-0 font-mono text-xs overflow-hidden"
              title={rawFilePath}
            >
              {dirPath && (
                <span className="text-[#8f8f8f] truncate max-w-[120px] sm:max-w-[180px] shrink-0">
                  {dirPath}/
                </span>
              )}
              <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
                {fileName || rawFilePath || 'file'}
              </span>
            </div>

            {/* Delta badge: +X -Y */}
            {(additions > 0 || deletions > 0) && (
              <div className="flex items-center space-x-1 font-mono text-[11px] tabular-nums font-semibold shrink-0 ml-1">
                {additions > 0 && (
                  <span className="text-[#16a34a] bg-[#f0fdf4] px-1 py-0.2 rounded border border-[#bbf7d0]">
                    +{additions}
                  </span>
                )}
                {deletions > 0 && (
                  <span className="text-[#dc2626] bg-[#fef2f2] px-1 py-0.2 rounded border border-[#fecaca]">
                    -{deletions}
                  </span>
                )}
              </div>
            )}

            {durationText && (
              <>
                <span className="text-[#a1a1aa] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] shrink-0 tabular-nums text-[11px]">
                  {durationText}
                </span>
              </>
            )}
          </div>
        ) : (
          /* Standard tool card */
          <div className="flex items-center space-x-1.5 min-w-0 flex-1 overflow-hidden">
            <button
              type="button"
              className="p-0.5 text-[#8f8f8f] hover:text-[#171717] rounded shrink-0 transition cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
              title={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
            >
              <ChevronRight
                className={`h-3 w-3 transition-transform duration-150 ${
                  isExpanded ? 'rotate-90 text-[#171717]' : ''
                }`}
              />
            </button>

            <span
              className="flex items-center shrink-0 select-none text-[#8f8f8f]"
              title="Инструмент"
            >
              <Wrench className="h-3.5 w-3.5" />
            </span>

            <span className="font-mono font-medium text-[#171717] dark:text-[#f0f6fc] shrink-0">
              {tool.toolName}{target ? ':' : ''}
            </span>

            {target && (
              <span
                title={fullTarget}
                className="font-mono text-[#52525b] dark:text-[#8b949e] truncate max-w-[260px] sm:max-w-[360px] md:max-w-[460px]"
              >
                {target}
              </span>
            )}

            {extraInfo && (
              <>
                <span className="text-[#a1a1aa] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] shrink-0 tabular-nums text-[11px]">
                  {extraInfo}
                </span>
              </>
            )}

            {durationText && (
              <>
                <span className="text-[#a1a1aa] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] shrink-0 tabular-nums text-[11px]">
                  {durationText}
                </span>
              </>
            )}
          </div>
        )}

        <div className="flex items-center space-x-2 shrink-0">
          <button
            type="button"
            onClick={handleCopyLogs}
            className={`p-1 rounded text-[#8f8f8f] hover:text-[#171717] hover:bg-white transition cursor-pointer ${
              isExpanded ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
            }`}
            title="Скопировать вывод"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>

          {renderStatusIndicator()}
        </div>
      </div>

      {/* Expanded Details */}
      {isExpanded && (
        <div className="p-3 border-t border-[#ebebeb] dark:border-[#21262d] space-y-2.5 font-mono text-xs bg-white dark:bg-[#0d1117]">
          {isReadOp ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              {tool.stdout ? (
                <div>
                  <span className="text-[10px] text-[#666666] block mb-1">
                    Прочитанные строки {lineRangeBadge ? `(${lineRangeBadge})` : ''}
                  </span>
                  {renderReadContent()}
                </div>
              ) : (
                tool.params &&
                Object.keys(tool.params).length > 0 && renderToolParams(params)
              )}
            </div>
          ) : isFileOp ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              {oldString !== undefined && newString !== undefined ? (
                <div>
                  <span className="text-[10px] text-[#666666] block mb-1">
                    {isFailed ? 'Запланированные изменения (не применены)' : 'Изменения файлов (Diff)'}
                  </span>
                  {renderUnifiedDiff()}
                </div>
              ) : newString ? (
                <div>
                  <span className="text-[10px] text-[#666666] block mb-1">
                    Содержимое файла
                  </span>
                  {renderFileContentPreview()}
                </div>
              ) : (
                tool.params &&
                Object.keys(tool.params).length > 0 && renderToolParams(params)
              )}
            </div>
          ) : isDiffTool && (oldString !== undefined || newString !== undefined) ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              <div>
                <span className="text-[10px] text-[#666666] block mb-1">
                  {isFailed ? 'Запланированные изменения (не применены)' : 'Изменения файлов (Diff)'}
                </span>
                {renderUnifiedDiff()}
              </div>
            </div>
          ) : (
            tool.params &&
            Object.keys(tool.params).length > 0 && renderToolParams(params)
          )}

          {tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] block mb-1">
                Стандартный вывод (stdout)
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[11px] text-[#171717] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] block mb-1">
                Трассировка ошибок (stderr)
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] border border-[#fecaca] text-[11px] text-[#991b1b] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}

          <div className="flex items-center justify-between pt-1 border-t border-[#f4f4f5] text-[10px] text-[#8f8f8f]">
            <span>Статус: {tool.status} {tool.exitCode !== undefined ? `(код ${tool.exitCode})` : ''}</span>
            <span className="tabular-nums">{tool.timestamp}</span>
          </div>
        </div>
      )}
    </div>
  );
}
