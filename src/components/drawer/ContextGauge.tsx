import { useState } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { KanbanTask } from '../../types';
import { Zap, Check, HardDrive } from 'lucide-react';

interface ContextGaugeProps {
  task: KanbanTask;
}

export default function ContextGauge({ task }: ContextGaugeProps) {
  const { contextStats, compactContext } = useDrawerStore();
  const [isCompacting, setIsCompacting] = useState(false);
  const [compactedSuccess, setCompactedSuccess] = useState(false);

  const stats = contextStats[task.id] || {
    totalTokens: 18400,
    maxTokens: 128000,
    percent: 14,
    turnCount: 4,
    compactedTurns: 0,
    tokensSaved: 0,
  };

  const handleCompact = async () => {
    setIsCompacting(true);
    await compactContext(task.id);
    setIsCompacting(false);
    setCompactedSuccess(true);
    setTimeout(() => setCompactedSuccess(false), 2000);
  };

  const getBarColor = (percent: number) => {
    if (percent > 80) return 'bg-rose-500 shadow-rose-500/50';
    if (percent > 60) return 'bg-cyber-amber shadow-amber-500/50';
    return 'bg-cyber-cyan shadow-cyan-500/50';
  };

  return (
    <div className="flex flex-col gap-1.5 p-2 rounded-lg bg-obsidian-950/70 border border-slate-800 text-xs">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-1.5 text-slate-300 font-mono text-[11px]">
          <HardDrive className="h-3 w-3 text-cyber-cyan shrink-0" />
          <span>Context Window:</span>
          <strong className="text-white">
            {stats.totalTokens.toLocaleString()}
          </strong>
          <span className="text-slate-500">/</span>
          <span className="text-slate-400">
            {stats.maxTokens.toLocaleString()} tokens
          </span>
          <span className="px-1 py-0.2 rounded bg-obsidian-850 border border-slate-750 text-slate-300 font-bold text-[10px]">
            {stats.percent}%
          </span>
        </div>

        {/* Compact Context Button */}
        <button
          onClick={handleCompact}
          disabled={isCompacting}
          className={`flex items-center space-x-1 px-2.5 py-1 rounded font-mono text-[11px] font-bold transition active:scale-95 cursor-pointer ${
            compactedSuccess
              ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
              : 'bg-cyber-amber/15 hover:bg-cyber-amber/25 text-amber-300 border border-amber-500/40 shadow-xs shadow-amber-950/30'
          }`}
          title="Fast zero-LLM evaporation of tool outputs >3 turns old"
        >
          {compactedSuccess ? (
            <>
              <Check className="h-3 w-3 text-emerald-400" />
              <span>Compacted!</span>
            </>
          ) : (
            <>
              <Zap
                className={`h-3 w-3 ${
                  isCompacting ? 'animate-spin' : 'fill-amber-300'
                }`}
              />
              <span>{isCompacting ? 'Compacting...' : '⚡ Compact Context'}</span>
            </>
          )}
        </button>
      </div>

      {/* Progress Bar */}
      <div className="h-1.5 w-full rounded-full bg-obsidian-800 overflow-hidden relative">
        <div
          className={`h-full transition-all duration-300 rounded-full shadow-sm ${getBarColor(
            stats.percent
          )}`}
          style={{ width: `${Math.min(100, stats.percent)}%` }}
        />
      </div>

      {stats.tokensSaved > 0 && (
        <div className="flex items-center justify-between text-[10px] text-slate-400 font-mono">
          <span className="text-cyber-emerald">
            ✓ Preserved immutable anchors (System prompt, Task prompt, last 2 turns)
          </span>
          <span className="text-cyber-amber font-semibold">
            Saved {stats.tokensSaved.toLocaleString()} tokens
          </span>
        </div>
      )}
    </div>
  );
}
