import { useEffect, useMemo, useState } from 'react';
import { useToolPolicyStore, type ToolMeta } from '../../../stores/toolPolicyStore';

export default function ToolSchemaEditor({ tool }: { tool: ToolMeta }) {
  const { toolCustomSchemas, setToolCustomSchema, resetToolCustomSchema } = useToolPolicyStore();

  const defaultSchema = useMemo(
    () => ({
      type: 'function',
      function: {
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters || { type: 'object', properties: {} },
      },
    }),
    [tool.name, tool.description, tool.parameters],
  );

  const defaultJsonString = useMemo(() => {
    return JSON.stringify(defaultSchema, null, 2);
  }, [defaultSchema]);

  const customSchema = toolCustomSchemas[tool.id];
  const isCustomized = customSchema !== undefined;

  const activeJsonString = useMemo(() => {
    if (customSchema !== undefined) {
      try {
        return typeof customSchema === 'string'
          ? JSON.stringify(JSON.parse(customSchema), null, 2)
          : JSON.stringify(customSchema, null, 2);
      } catch {
        return typeof customSchema === 'string' ? customSchema : JSON.stringify(customSchema, null, 2);
      }
    }
    return defaultJsonString;
  }, [customSchema, defaultJsonString]);

  const [jsonText, setJsonText] = useState(activeJsonString);
  const [parseError, setParseError] = useState<string | null>(null);
  const [savedFeedback, setSavedFeedback] = useState(false);

  useEffect(() => {
    setJsonText(activeJsonString);
    setParseError(null);
  }, [activeJsonString]);

  const handleFormat = (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const parsed = JSON.parse(jsonText);
      const formatted = JSON.stringify(parsed, null, 2);
      setJsonText(formatted);
      setParseError(null);
    } catch (err) {
      setParseError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleSave = (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const parsed = JSON.parse(jsonText);
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        setParseError('JSON должен быть объектом { "type": "function", "function": { ... } }');
        return;
      }
      setToolCustomSchema(tool.id, parsed);
      setParseError(null);
      setSavedFeedback(true);
      setTimeout(() => setSavedFeedback(false), 2000);
    } catch (err) {
      setParseError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleReset = (e: React.MouseEvent) => {
    e.stopPropagation();
    resetToolCustomSchema(tool.id);
    setJsonText(defaultJsonString);
    setParseError(null);
  };

  return (
    <div
      onClick={(e) => e.stopPropagation()}
      className="mt-2.5 p-2.5 rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#f9f9fb] dark:bg-[#0b0b0e] space-y-2 select-none"
    >
      <div className="flex items-center justify-between gap-2 border-b border-[#ebebeb] dark:border-[#212126] pb-1.5 flex-wrap">
        <div className="flex items-center gap-2">
          <span className="text-[10px] font-mono uppercase tracking-wider text-[#666666] dark:text-[#71717a] font-semibold">
            Схема инструмента (JSON)
          </span>
          {isCustomized && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/30">
              Кастомная схема
            </span>
          )}
          {savedFeedback && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30 animate-pulse">
              Сохранено
            </span>
          )}
        </div>

        <div className="flex items-center gap-1.5 flex-wrap">
          <button
            type="button"
            onClick={handleFormat}
            className="px-2 py-0.5 rounded border border-[#d4d4d8] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] text-[#171717] dark:text-[#f4f4f5] text-[10px] font-mono hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer"
          >
            Форматировать
          </button>
          <button
            type="button"
            onClick={handleSave}
            className="px-2 py-0.5 rounded border border-[#171717] dark:border-[#f4f4f5] bg-[#171717] dark:bg-[#f4f4f5] text-white dark:text-[#111114] text-[10px] font-mono font-medium hover:opacity-90 transition cursor-pointer"
          >
            Сохранить JSON
          </button>
          {isCustomized && (
            <button
              type="button"
              onClick={handleReset}
              className="px-2 py-0.5 rounded border border-[#fecaca] dark:border-[#da3633]/40 bg-[#fef2f2] dark:bg-[#3b1219] text-[#991b1b] dark:text-[#ff7b72] text-[10px] font-mono hover:opacity-90 transition cursor-pointer"
            >
              Сбросить по умолчанию
            </button>
          )}
        </div>
      </div>

      <div className="pt-1 space-y-1.5">
        <textarea
          value={jsonText}
          onChange={(e) => {
            setJsonText(e.target.value);
            if (parseError) setParseError(null);
          }}
          rows={11}
          spellCheck={false}
          className="w-full rounded border border-[#d4d4d8] dark:border-[#2c2c33] bg-white dark:bg-[#111114] text-[#24292f] dark:text-[#a1a1aa] font-mono text-xs p-2.5 focus:outline-hidden focus:border-[#171717] dark:focus:border-[#58a6ff] leading-relaxed resize-y select-text"
          placeholder='{ "type": "function", "function": { ... } }'
        />

        {parseError && (
          <p className="text-[11px] font-mono text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-900/50 rounded px-2.5 py-1.5">
            Ошибка: {parseError}
          </p>
        )}
      </div>
    </div>
  );
}
