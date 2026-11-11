import { useEffect, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import {
  applyShellToExecuteCommandSchema,
  useToolPolicyStore,
  TOOL_PARAM_DEFS,
  type ExecuteShell,
} from '../../../stores/toolPolicyStore';
import { useTranslation, type TranslationKey } from '../../../stores/languageStore';

export function PolicyToggle({
  enabled,
  onToggle,
  title,
  disabled,
}: {
  enabled: boolean;
  onToggle: () => void;
  title: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        if (!disabled) onToggle();
      }}
      disabled={disabled}
      aria-label={title}
      aria-pressed={enabled}
      className={`relative w-9 h-5 rounded-full transition-colors cursor-pointer shrink-0 ${
        enabled ? 'bg-[#171717] dark:bg-[#f4f4f5]' : 'bg-[#e4e4e7] dark:bg-[#2c2c33]'
      } ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
    >
      <span
        className={`absolute top-0.5 h-4 w-4 rounded-full bg-white dark:bg-[#111114] shadow transition-all ${
          enabled ? 'left-[18px]' : 'left-0.5'
        }`}
      />
    </button>
  );
}

function ToolParamRow({
  toolId,
  param,
}: {
  toolId: string;
  param: {
    key: string;
    label: string;
    defaultValue: number;
    min: number;
    max: number;
    step: number;
    unit: string;
  };
}) {
  const { toolSettings, setToolSetting } = useToolPolicyStore(
    useShallow((s) => ({
      toolSettings: s.toolSettings,
      setToolSetting: s.setToolSetting,
    })),
  );
  const { t } = useTranslation();
  const rawValue = toolSettings[toolId]?.[param.key];
  const currentValue = typeof rawValue === 'number' ? rawValue : param.defaultValue;
  const [draft, setDraft] = useState(String(currentValue));

  const paramLabelKey = `toolParam.${param.key}.label` as TranslationKey;
  const paramUnitKey = `toolParam.${param.key}.unit` as TranslationKey;
  const paramLabel = t(paramLabelKey) !== paramLabelKey ? t(paramLabelKey) : param.label;
  const paramUnit = t(paramUnitKey) !== paramUnitKey ? t(paramUnitKey) : param.unit;

  useEffect(() => {
    setDraft(String(currentValue));
  }, [currentValue]);

  const commitValue = (val: number) => {
    const clamped = Math.max(param.min, Math.min(param.max, val));
    setDraft(String(clamped));
    setToolSetting(toolId, param.key, clamped);
  };

  const handleStep = (delta: number) => {
    commitValue(currentValue + delta);
  };

  const handleBlur = () => {
    const parsed = parseInt(draft, 10);
    if (isNaN(parsed)) {
      setDraft(String(currentValue));
    } else {
      commitValue(parsed);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      (e.target as HTMLInputElement).blur();
    }
    if (e.key === 'Escape') {
      setDraft(String(currentValue));
      (e.target as HTMLInputElement).blur();
    }
  };

  return (
    <div className="flex items-center justify-between gap-3 text-xs">
      <div className="min-w-0 flex-1">
        <span className="text-[11px] font-mono text-[#171717] dark:text-[#f4f4f5] block">
          {paramLabel}
        </span>
        <span className="text-[10px] text-[#8f8f8f] dark:text-[#71717a] font-mono block">
          {t('toolPolicy.rangeStep', { min: param.min, max: param.max, step: param.step })}
        </span>
      </div>

      <div className="flex items-center gap-1.5 shrink-0">
        <button
          type="button"
          disabled={currentValue <= param.min}
          onClick={() => handleStep(-param.step)}
          className="w-6 h-6 flex items-center justify-center rounded border border-[#d4d4d8] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] text-xs font-mono font-bold transition select-none cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
          aria-label={t('toolPolicy.decreaseBy', { step: param.step })}
        >
          -
        </button>

        <input
          type="number"
          min={param.min}
          max={param.max}
          step={param.step}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={handleBlur}
          onKeyDown={handleKeyDown}
          className="w-16 h-6 rounded border border-[#d4d4d8] dark:border-[#2c2c33] bg-white dark:bg-[#111114] text-[#171717] dark:text-[#f4f4f5] font-mono text-xs tabular-nums text-center px-1 focus:outline-hidden focus:border-[#171717] dark:focus:border-[#58a6ff]"
        />

        <button
          type="button"
          disabled={currentValue >= param.max}
          onClick={() => handleStep(param.step)}
          className="w-6 h-6 flex items-center justify-center rounded border border-[#d4d4d8] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] text-xs font-mono font-bold transition select-none cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
          aria-label={t('toolPolicy.increaseBy', { step: param.step })}
        >
          +
        </button>

        <span className="text-[10px] text-[#666666] dark:text-[#71717a] font-mono min-w-[42px] text-left">
          {paramUnit}
        </span>
      </div>
    </div>
  );
}

function ShellSelectorRow() {
  const { toolSettings, setToolSetting, toolCustomSchemas, setToolCustomSchema } = useToolPolicyStore(
    useShallow((s) => ({
      toolSettings: s.toolSettings,
      setToolSetting: s.setToolSetting,
      toolCustomSchemas: s.toolCustomSchemas,
      setToolCustomSchema: s.setToolCustomSchema,
    })),
  );
  const selectedShell = ((toolSettings['execute_command']?.shell as string) || 'bash') as ExecuteShell;

  const handleShellChange = (newShell: ExecuteShell) => {
    setToolSetting('execute_command', 'shell', newShell);
    const custom = toolCustomSchemas['execute_command'];
    if (custom !== undefined) {
      try {
        const parsed = typeof custom === 'string' ? JSON.parse(custom) : { ...custom };
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed) && parsed.function) {
          setToolCustomSchema('execute_command', applyShellToExecuteCommandSchema(parsed as Record<string, unknown>, newShell));
        }
      } catch (err) {
        // Ignore draft syntax errors
        console.debug('[ToolParamsEditor] custom schema JSON parse skipped:', err);
      }
    }
  };

  return (
    <div className="flex items-center justify-between gap-3 text-xs pt-1.5 border-t border-[#ebebeb] dark:border-[#212126]">
      <div className="min-w-0 flex-1">
        <span className="text-[11px] font-mono text-[#171717] dark:text-[#f4f4f5] block">
          Оболочка выполнения (Shell)
        </span>
        <span className="text-[10px] text-[#8f8f8f] dark:text-[#71717a] font-mono block">
          {selectedShell === 'bash'
            ? 'Git Bash (Linux синтаксис, bash -c)'
            : selectedShell === 'pwsh'
            ? 'PowerShell (powershell.exe -Command)'
            : 'Command Prompt (cmd.exe /C)'}
        </span>
      </div>

      <div className="flex items-center gap-1 bg-[#f4f4f5] dark:bg-[#18181b] p-0.5 rounded border border-[#e4e4e7] dark:border-[#27272a] text-[10px] font-mono shrink-0">
        {(['bash', 'pwsh', 'cmd'] as const).map((shell) => {
          const active = selectedShell === shell;
          return (
            <button
              key={shell}
              type="button"
              onClick={() => handleShellChange(shell)}
              className={`px-2.5 py-0.5 rounded transition cursor-pointer font-medium ${
                active
                  ? 'bg-white dark:bg-[#27272a] text-[#171717] dark:text-[#f4f4f5] shadow-xs border border-[#d4d4d8] dark:border-[#3f3f46]'
                  : 'text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
              }`}
            >
              {shell}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export default function ToolParamsEditor({ toolId }: { toolId: string }) {
  const paramDefs = TOOL_PARAM_DEFS[toolId];
  const { t } = useTranslation();

  if (!paramDefs || paramDefs.length === 0) return null;

  return (
    <div
      onClick={(e) => e.stopPropagation()}
      className="mt-2.5 p-2.5 rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#f9f9fb] dark:bg-[#0b0b0e] space-y-2 select-none"
    >
      <div className="flex items-center justify-between text-[10px] font-mono uppercase tracking-wider text-[#666666] dark:text-[#71717a] font-semibold border-b border-[#ebebeb] dark:border-[#212126] pb-1.5">
        <span>{t('toolPolicy.toolParams')}</span>
        <span className="text-[9px] text-[#8f8f8f] dark:text-[#52525b] lowercase font-normal">
          {t('toolPolicy.savedInPolicies')}
        </span>
      </div>
      {paramDefs.map((param) => (
        <ToolParamRow key={param.key} toolId={toolId} param={param} />
      ))}
      {toolId === 'execute_command' && <ShellSelectorRow />}
    </div>
  );
}
