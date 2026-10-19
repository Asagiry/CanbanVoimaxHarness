import { useState, useRef, useEffect, useMemo } from 'react';
import AgentIcon from './AgentIcon';
import { getDeterministicAgentColor } from '../../lib/agentPalette';
import { useCortexStore } from '../../stores/cortexStore';
import { ChevronDown, Check, Plus, UserX } from 'lucide-react';

export interface PersonaItem {
  role: string;
  icon?: string;
  color?: string;
  isBuiltin?: boolean;
  is_builtin?: boolean;
  hasPrompt?: boolean;
}

export interface PersonaSelectProps {
  value: string;
  onChange: (value: string) => void;
  personas?: PersonaItem[];
  disabled?: boolean;
  allowEmpty?: boolean;
  emptyLabel?: string;
  allowNewAgent?: boolean;
  newAgentOptionValue?: string;
  className?: string;
  placeholder?: string;
  compact?: boolean;
}

export default function PersonaSelect({
  value,
  onChange,
  personas: passedPersonas,
  disabled = false,
  allowEmpty = false,
  emptyLabel = 'без агента',
  allowNewAgent = false,
  newAgentOptionValue = '+new',
  className = '',
  placeholder = 'Выберите агента...',
  compact = false,
}: PersonaSelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const { agentPersonas } = useCortexStore();

  const personas = useMemo<PersonaItem[]>(() => {
    if (passedPersonas && passedPersonas.length > 0) {
      return passedPersonas;
    }
    return agentPersonas;
  }, [passedPersonas, agentPersonas]);

  // Click outside listener
  useEffect(() => {
    if (!isOpen) return;

    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        setIsOpen(false);
      }
    }

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  const isNewAgent = allowNewAgent && value === newAgentOptionValue;
  const selected = personas.find((p) => p.role === value);
  const isBuiltin = selected ? (selected.role === 'cvh-dev' || Boolean(selected.isBuiltin || selected.is_builtin)) : false;
  const color = selected?.color || (selected?.role ? getDeterministicAgentColor(selected.role) : '#64748b');
  const icon = selected?.icon || (selected?.role === 'cvh-dev' ? 'Cpu' : 'Bot');

  return (
    <div ref={containerRef} className={`relative inline-block ${className || 'w-full'}`}>
      {/* Trigger Button */}
      <button
        type="button"
        disabled={disabled}
        onClick={() => !disabled && setIsOpen((prev) => !prev)}
        className={`w-full flex items-center justify-between gap-1.5 ${
          compact ? 'px-1.5 py-1 text-[11px]' : 'px-2.5 py-1.5 text-xs'
        } rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] hover:border-[#d4d4d8] dark:hover:border-[#484f58] transition cursor-pointer font-mono disabled:opacity-50 disabled:cursor-not-allowed select-none min-w-0 ${
          isOpen ? 'border-[#171717] dark:border-[#f0f6fc]' : ''
        }`}
      >
        <div className="flex items-center gap-1.5 min-w-0 flex-1 text-left">
          {isNewAgent ? (
            <div className="flex items-center gap-1.5 min-w-0 flex-1">
              <Plus className="h-3.5 w-3.5 text-indigo-500 shrink-0" />
              <span className="text-indigo-600 dark:text-indigo-400 font-medium truncate">
                + Новый агент
              </span>
            </div>
          ) : !value && allowEmpty ? (
            <div className="flex items-center gap-1.5 min-w-0 flex-1 text-[#8f8f8f] dark:text-[#8b949e]">
              <UserX className="h-3.5 w-3.5 shrink-0" />
              <span className="font-sans truncate">{emptyLabel}</span>
            </div>
          ) : selected ? (
            <div className="flex items-center gap-1.5 min-w-0 flex-1">
              <div className="h-4 w-4 rounded flex items-center justify-center shrink-0" style={{ color }}>
                <AgentIcon name={icon} className="h-3.5 w-3.5" />
              </div>
              <span className="font-semibold truncate leading-none min-w-0 flex-1" style={{ color }} title={selected.role}>
                {selected.role}
              </span>
              {isBuiltin && (
                <span className="px-1 py-0.2 rounded text-[9px] font-sans font-medium bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800 shrink-0">
                  {compact ? 'встр.' : 'встроенный'}
                </span>
              )}
            </div>
          ) : value ? (
            <div className="flex items-center gap-1.5 min-w-0 flex-1">
              <AgentIcon name="Bot" className="h-3.5 w-3.5 text-[#8f8f8f] shrink-0" />
              <span className="font-semibold truncate leading-none text-[#171717] dark:text-[#f0f6fc] min-w-0 flex-1" title={value}>
                {value}
              </span>
            </div>
          ) : (
            <span className="text-[#8f8f8f] dark:text-[#8b949e] font-sans truncate">
              {placeholder}
            </span>
          )}
        </div>

        <ChevronDown
          className={`h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e] shrink-0 transition-transform ${
            isOpen ? 'rotate-180' : ''
          }`}
        />
      </button>

      {/* Dropdown Menu */}
      {isOpen && (
        <div className="absolute z-[90] left-0 right-0 mt-1 min-w-[200px] max-h-60 overflow-y-auto rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] shadow-md py-1 text-xs font-mono">
          {allowEmpty && (
            <button
              type="button"
              onClick={() => {
                onChange('');
                setIsOpen(false);
              }}
              className={`w-full flex items-center justify-between gap-2 px-2.5 py-1.5 hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-left cursor-pointer transition select-none ${
                !value ? 'bg-[#f0f0f2] dark:bg-[#21262d]' : ''
              }`}
            >
              <div className="flex items-center gap-2 min-w-0 flex-1 text-[#8f8f8f] dark:text-[#8b949e]">
                <UserX className="h-3.5 w-3.5 shrink-0" />
                <span className="font-sans">{emptyLabel}</span>
              </div>
              {!value && <Check className="h-3.5 w-3.5 text-[#171717] dark:text-[#f0f6fc] shrink-0" />}
            </button>
          )}

          {personas.map((p) => {
            const pIsBuiltin = p.role === 'cvh-dev' || Boolean(p.isBuiltin || p.is_builtin);
            const pColor = p.color || getDeterministicAgentColor(p.role);
            const pIcon = p.icon || (p.role === 'cvh-dev' ? 'Cpu' : 'Bot');
            const isCurrent = p.role === value;

            return (
              <button
                key={p.role}
                type="button"
                onClick={() => {
                  onChange(p.role);
                  setIsOpen(false);
                }}
                className={`w-full flex items-center justify-between gap-2 px-2.5 py-1.5 hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-left cursor-pointer transition select-none ${
                  isCurrent ? 'bg-[#f0f0f2] dark:bg-[#21262d]' : ''
                }`}
              >
                <div className="flex items-center gap-2 min-w-0 flex-1">
                  <div className="h-4 w-4 rounded flex items-center justify-center shrink-0" style={{ color: pColor }}>
                    <AgentIcon name={pIcon} className="h-3.5 w-3.5" />
                  </div>
                  <span className="font-semibold truncate" style={{ color: pColor }}>
                    {p.role}
                  </span>
                  {pIsBuiltin && (
                    <span className="px-1 py-0.2 rounded text-[9px] font-sans font-medium bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800 shrink-0">
                      встроенный
                    </span>
                  )}
                </div>
                {isCurrent && <Check className="h-3.5 w-3.5 text-[#171717] dark:text-[#f0f6fc] shrink-0" />}
              </button>
            );
          })}

          {allowNewAgent && (
            <div className="border-t border-[#ebebeb] dark:border-[#21262d] mt-1 pt-1">
              <button
                type="button"
                onClick={() => {
                  onChange(newAgentOptionValue);
                  setIsOpen(false);
                }}
                className={`w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-left cursor-pointer transition select-none text-indigo-600 dark:text-indigo-400 font-medium ${
                  isNewAgent ? 'bg-indigo-50 dark:bg-indigo-950/40' : ''
                }`}
              >
                <Plus className="h-3.5 w-3.5 shrink-0" />
                <span>+ Новый агент</span>
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
