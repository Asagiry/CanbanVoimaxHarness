import { Draggable } from '@hello-pangea/dnd';
import { KanbanTask, TaskVerdict } from '../../types';
import { formatDuration } from '../../lib/utils';
import { useDrawerStore } from '../../stores/drawerStore';

interface KanbanCardProps {
  task: KanbanTask;
  index: number;
}

const PERSONA_TITLES: Record<string, string> = {
  developer: 'Разработчик',
  reviewer: 'Ревьюер',
  retro: 'Ретро-инженер',
  artisan: 'Инженер UI',
};

export default function KanbanCard({ task, index }: KanbanCardProps) {
  const { openDrawer } = useDrawerStore();

  const getVerdictBadge = (verdict?: TaskVerdict) => {
    if (!verdict || verdict === 'NONE') return null;
    switch (verdict) {
      case 'APPROVED':
        return (
          <span className="text-[10px] font-mono text-emerald-400">
            Одобрено
          </span>
        );
      case 'APPROVED_WITH_CAVEATS':
        return (
          <span className="text-[10px] font-mono text-amber-400">
            С замечаниями
          </span>
        );
      case 'CHANGES_REQUESTED':
        return (
          <span className="text-[10px] font-mono text-rose-400">
            Требуются правки
          </span>
        );
    }
  };

  const personaLabel = PERSONA_TITLES[task.persona.toLowerCase()] || task.persona;

  return (
    <Draggable draggableId={task.id} index={index}>
      {(provided, snapshot) => (
        <div
          ref={provided.innerRef}
          {...provided.draggableProps}
          {...provided.dragHandleProps}
          onClick={() => openDrawer(task.id)}
          className={`group relative mb-2 rounded border p-2.5 cursor-pointer select-none transition duration-150 active:scale-[0.99] ${
            snapshot.isDragging
              ? 'border-slate-500 bg-obsidian-800 shadow-md'
              : 'border-obsidian-800 bg-obsidian-900 hover:border-obsidian-700 hover:bg-obsidian-850'
          }`}
        >
          {/* Header row: ID + Verdict */}
          <div className="flex items-center justify-between mb-1.5 text-xs">
            <span className="font-mono text-[11px] font-semibold text-slate-400">
              {task.id}
            </span>
            {getVerdictBadge(task.verdict)}
          </div>

          {/* Title */}
          <h4 className="text-xs font-medium text-slate-200 group-hover:text-white leading-snug line-clamp-2 mb-2 font-sans">
            {task.title}
          </h4>

          {/* Footer row: Executor & Status/Timer */}
          <div className="flex items-center justify-between pt-1.5 border-t border-obsidian-800/80 text-[10px] font-mono text-slate-400">
            <span className="truncate max-w-[130px] text-slate-400">
              {personaLabel}
            </span>

            <div className="flex items-center space-x-1 tabular-nums">
              {task.status === 'waiting_review' ? (
                <span className="text-amber-400 font-sans">На проверке</span>
              ) : task.status === 'blocked' ? (
                <span className="text-rose-400 font-sans">Заблокирован</span>
              ) : task.isTimerRunning ? (
                <span className="text-emerald-400">
                  {formatDuration(task.activeStageDuration)}
                </span>
              ) : task.status === 'done' ? (
                <span className="text-slate-500 font-sans">Готово</span>
              ) : (
                <span className="text-slate-500 font-sans">В ожидании</span>
              )}
            </div>
          </div>
        </div>
      )}
    </Draggable>
  );
}
