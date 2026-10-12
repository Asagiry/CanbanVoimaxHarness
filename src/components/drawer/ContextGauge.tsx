import { useState } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { Zap, Flame, Cpu } from 'lucide-react';

interface ContextGaugeProps {
  taskId: string;
}

export default function ContextGauge({ taskId }: ContextGaugeProps) {
  const { streamEntries, toolExecutions, compactTaskContext } = useDrawerStore();
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
    if (pct < 50) return 'bg-cyber-cyan';
    if (pct <= 75) return 'bg-cyber-amber';
    return 'bg-cyber-crimson';
  };

  const getTextColor = (pct: number) => {
    if (pct < 50) return 'text-cyber-cyan';
    if (pct <= 75) return 'text-cyber-amber';
    return 'text-cyber-crimson';
  };

  const handleCompact = async () => {
    const { savedTokens } = await compactTaskContext(taskId);
    setEvaporatedBadge(`🔥 ~${savedTokens.toLocaleString()} tokens saved!`);
    setTimeout(() => {
      setEvaporatedBadge(null);
    }, 3000);
  };

  return (
    <div className="flex flex-col space-y-1.5 p-2 rounded-md bg-obsidian-950 border border-obsidian-800 text-xs font-mono">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-1.5 text-slate-300">
          <Cpu className="h-3.5 w-3.5 text-cyber-cyan shrink-0" />
          <span className="font-semibold text-[11px] text-slate-300">Context Window:</span>
          <span className={`text-[11px] font-bold tabular-nums ${getTextColor(fillPercentage)}`}>
            {estimatedTokens.toLocaleString()} / {maxTokens.toLocaleString()} tokens ({fillPercentage}%)
          </span>
        </div>

        {/* Compact Button & Temporary Badge */}
        <div className="flex items-center space-x-1.5">
          {evaporatedBadge && (
            <span className="flex items-center space-x-1 px-1.5 py-0.2 rounded text-[10px] font-mono font-bold bg-amber-950/40 text-cyber-amber border border-amber-500/40 animate-in fade-in tabular-nums">
              <Flame className="h-2.5 w-2.5 text-cyber-amber animate-pulse shrink-0" />
              <span>{evaporatedBadge}</span>
            </span>
          )}

          <button
            onClick={handleCompact}
            className="flex items-center space-x-1 px-2 py-0.5 rounded-sm bg-obsidian-900 hover:bg-obsidian-850 text-cyber-amber hover:text-amber-300 border border-amber-500/30 hover:border-amber-400 text-[10px] font-mono font-bold transition active:scale-[0.98] cursor-pointer select-none"
            title="Evaporate old tool logs (>500 chars) older than 3 turns to conserve LLM context"
          >
            <Zap className="h-2.5 w-2.5 fill-cyber-amber shrink-0" />
            <span>⚡ Compact Context</span>
          </button>
        </div>
      </div>

      {/* Sleek Progress Bar */}
      <div className="h-1 w-full rounded-full bg-obsidian-800 overflow-hidden">
        <div
          className={`h-full transition-all duration-300 ${getFillColor(fillPercentage)}`}
          style={{ width: `${Math.max(2, fillPercentage)}%` }}
        />
      </div>
    </div>
  );
}
