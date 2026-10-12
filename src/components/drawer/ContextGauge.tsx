import { useState } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { Zap, Flame, Cpu } from 'lucide-react';

interface ContextGaugeProps {
  taskId: string;
}

export default function ContextGauge({ taskId }: ContextGaugeProps) {
  const { streamEntries, toolExecutions, evaporateOldLogs } = useDrawerStore();
  const { tasks, columns } = useBoardStore();
  const [evaporatedBadge, setEvaporatedBadge] = useState<string | null>(null);

  const task = tasks.find((t) => t.id === taskId);
  const column = columns.find((c) => c.id === task?.columnId);
  const maxTokens = column?.contextWindow || 128000;

  const currentStream = streamEntries[taskId] || { thoughts: [], outputs: [] };
  const currentTools = toolExecutions[taskId] || [];

  // Calculate dynamic token usage
  const thoughtsLen = currentStream.thoughts.reduce((acc, t) => acc + t.content.length, 0);
  const outputsLen = currentStream.outputs.reduce((acc, o) => acc + o.content.length, 0);
  const toolsLen = currentTools.reduce((acc, tool) => {
    const stdoutLen = tool.stdout?.length || 0;
    const stderrLen = tool.stderr?.length || 0;
    const paramsLen = JSON.stringify(tool.params || {}).length;
    return acc + stdoutLen + stderrLen + paramsLen;
  }, 0);

  const estimatedTokens = Math.min(
    maxTokens,
    Math.round(8500 + (thoughtsLen + outputsLen + toolsLen) / 3.8)
  );

  const fillPercentage = Math.min(100, Math.round((estimatedTokens / maxTokens) * 100));

  const getFillColor = (pct: number) => {
    if (pct < 50) return 'bg-cyber-emerald';
    if (pct <= 75) return 'bg-cyber-amber';
    return 'bg-rose-500';
  };

  const getTextColor = (pct: number) => {
    if (pct < 50) return 'text-cyber-emerald';
    if (pct <= 75) return 'text-cyber-amber';
    return 'text-rose-400';
  };

  const handleCompact = () => {
    const { savedTokens } = evaporateOldLogs(taskId);
    setEvaporatedBadge(`🔥 Испарено ~${savedTokens.toLocaleString()} токенов логов!`);
    setTimeout(() => {
      setEvaporatedBadge(null);
    }, 3500);
  };

  return (
    <div className="flex flex-col space-y-1.5 p-2.5 rounded-lg bg-obsidian-950/70 border border-slate-800 text-xs font-mono">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-1.5 text-slate-300">
          <Cpu className="h-3.5 w-3.5 text-cyber-cyan" />
          <span className="font-semibold text-[11px] text-slate-300">Context Window:</span>
          <span className={`text-[11px] font-bold tabular-nums ${getTextColor(fillPercentage)}`}>
            {estimatedTokens.toLocaleString()} / {maxTokens.toLocaleString()} tokens ({fillPercentage}%)
          </span>
        </div>

        {/* Compact Button & Temporary Badge */}
        <div className="flex items-center space-x-2">
          {evaporatedBadge && (
            <span className="flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] font-bold bg-amber-500/20 text-cyber-amber border border-amber-500/40 animate-in fade-in zoom-in-95">
              <Flame className="h-3 w-3 text-cyber-amber animate-pulse" />
              <span>{evaporatedBadge}</span>
            </span>
          )}

          <button
            onClick={handleCompact}
            className="flex items-center space-x-1 px-2 py-1 rounded bg-obsidian-850 hover:bg-obsidian-800 text-cyber-amber hover:text-amber-300 border border-amber-500/30 hover:border-amber-400 text-[10px] font-bold transition active:scale-95 shadow-xs cursor-pointer select-none"
            title="Evaporate old tool logs (>500 chars) older than 3 turns to conserve LLM context"
          >
            <Zap className="h-3 w-3 fill-cyber-amber" />
            <span>⚡ Compact Context (Испарить логи)</span>
          </button>
        </div>
      </div>

      {/* Progress Bar */}
      <div className="h-1.5 w-full rounded-full bg-obsidian-800 overflow-hidden">
        <div
          className={`h-full transition-all duration-300 ${getFillColor(fillPercentage)}`}
          style={{ width: `${Math.max(4, fillPercentage)}%` }}
        />
      </div>
    </div>
  );
}
