import { Fragment, useState } from 'react';
import { ChevronDown, Trash2 } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';
import { PolicyToggle } from './ToolParamsEditor';
import {
  NAME_PATTERN,
  applyAgentToolToggle,
  saveCustomToolIpc,
  type AgentMatrixItem,
  type CustomTool,
  type StageColumnRef,
} from './toolPolicyConstants';

export function CustomToolRows({
  customTools,
  agents,
  columns,
  expandedToolId,
  onToggleExpanded,
  isToolAllowed,
  setStageTool,
  onDeleteCustomTool,
}: {
  customTools: CustomTool[];
  agents: AgentMatrixItem[];
  columns: ReadonlyArray<StageColumnRef>;
  expandedToolId: string | null;
  onToggleExpanded: (id: string) => void;
  isToolAllowed: (toolId: string, stage: string) => boolean;
  setStageTool: (stage: string, toolId: string, enabled: boolean) => void;
  onDeleteCustomTool: (name: string) => Promise<void>;
}) {
  const { t } = useTranslation();

  return (
    <>
      {customTools.map((customTool) => {
        const customId = `custom:${customTool.name}`;
        const isExpanded = expandedToolId === customId;

        return (
          <Fragment key={customId}>
            <tr className="hover:bg-[#fafafa] dark:hover:bg-[#1e1e23] transition text-[#171717] dark:text-[#f4f4f5]">
              <td className="py-2 px-3 align-middle">
                <div className="flex items-start gap-1.5 text-left w-full">
                  <button
                    type="button"
                    onClick={() => onToggleExpanded(customId)}
                    className="flex items-start gap-1.5 text-left cursor-pointer min-w-0 flex-1 group"
                  >
                    <ChevronDown
                      className={`w-3.5 h-3.5 shrink-0 mt-0.5 text-[#8f8f8f] dark:text-[#71717a] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-1.5">
                        <span className="font-medium text-[#171717] dark:text-[#f4f4f5] leading-tight group-hover:underline underline-offset-2">
                          {customTool.name}
                        </span>
                        <span className="rounded border border-[#ebebeb] dark:border-[#2c2c33] bg-[#f4f4f5] dark:bg-[#17171b] px-1 py-px text-[10px] leading-none text-[#666666] dark:text-[#71717a] align-middle">
                          {t('toolPolicy.customBadge')}
                        </span>
                      </div>
                      <div className="text-[10px] text-[#8f8f8f] dark:text-[#71717a] font-mono leading-tight mt-0.5 truncate">
                        {customTool.description}
                      </div>
                    </div>
                  </button>
                </div>
              </td>

              <td className="py-2 px-2 text-right text-[#8f8f8f] tabular-nums text-[11px] align-middle">—</td>

              {agents.map((agent) => {
                const isCustomAllowed = isToolAllowed(customTool.name, agent.role);
                return (
                  <td key={agent.role} className="py-2 px-2 text-center border-l border-[#ebebeb] dark:border-[#212126] align-middle">
                    <span className="inline-flex justify-center pt-0.5">
                      <PolicyToggle
                        enabled={isCustomAllowed}
                        onToggle={() => applyAgentToolToggle(agent.role, customTool.name, !isCustomAllowed, columns, setStageTool)}
                        title={t('toolPolicy.toolAgentStatus', {
                          tool: customTool.name,
                          agent: agent.role,
                          status: isCustomAllowed ? t('toolPolicy.statusEnabled') : t('toolPolicy.statusDisabled'),
                        })}
                      />
                    </span>
                  </td>
                );
              })}
            </tr>

            {isExpanded && (
              <tr className="bg-[#f9f9fb] dark:bg-[#0b0b0e] border-b border-[#ebebeb] dark:border-[#212126]">
                <td colSpan={2 + agents.length} className="p-3">
                  <div className="space-y-1.5 text-xs font-mono">
                    <div className="flex items-center justify-between">
                      <span className="text-[#666666] dark:text-[#71717a]">
                        {t('toolPolicy.commandPrefix')} <code className="bg-[#f4f4f5] dark:bg-[#232328] px-1 py-0.5 rounded">{customTool.command}</code>
                      </span>
                      <button
                        type="button"
                        onClick={() => void onDeleteCustomTool(customTool.name)}
                        className="px-2 py-1 rounded text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/40 text-xs flex items-center gap-1 cursor-pointer"
                      >
                        <Trash2 className="h-3 w-3" />
                        {t('common.delete')}
                      </button>
                    </div>
                  </div>
                </td>
              </tr>
            )}
          </Fragment>
        );
      })}
    </>
  );
}

export default function CustomToolForm({
  customLoading,
  customListError,
  customFormError,
  setCustomFormError,
  onToolSaved,
}: {
  customLoading: boolean;
  customListError: string | null;
  customFormError: string | null;
  setCustomFormError: (err: string | null) => void;
  onToolSaved: () => Promise<void>;
}) {
  const { t } = useTranslation();

  const [customName, setCustomName] = useState('');
  const [customDescription, setCustomDescription] = useState('');
  const [customCommand, setCustomCommand] = useState('');
  const [customParamsText, setCustomParamsText] = useState('{}');
  const [customSaving, setCustomSaving] = useState(false);

  const handleAddCustomTool = async (e: React.FormEvent) => {
    e.preventDefault();
    setCustomFormError(null);

    const name = customName.trim();
    const description = customDescription.trim();
    const command = customCommand.trim();
    const paramsRaw = customParamsText.trim() === '' ? '{}' : customParamsText.trim();

    if (!NAME_PATTERN.test(name)) {
      setCustomFormError(t('toolPolicy.errNamePattern'));
      return;
    }
    if (!description) {
      setCustomFormError(t('toolPolicy.errDescRequired'));
      return;
    }
    if (!command) {
      setCustomFormError(t('toolPolicy.errCommandRequired'));
      return;
    }
    let parameters: Record<string, unknown>;
    try {
      const parsed: unknown = JSON.parse(paramsRaw);
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        setCustomFormError(t('toolPolicy.errParamsObject'));
        return;
      }
      parameters = parsed as Record<string, unknown>;
    } catch (err) {
      setCustomFormError(t('toolPolicy.errParamsJson', { error: err instanceof Error ? err.message : String(err) }));
      return;
    }

    setCustomSaving(true);
    try {
      await saveCustomToolIpc({ name, description, parameters, command });
      setCustomName('');
      setCustomDescription('');
      setCustomCommand('');
      setCustomParamsText('{}');
      await onToolSaved();
    } catch (err) {
      setCustomFormError(err instanceof Error ? err.message : String(err));
    } finally {
      setCustomSaving(false);
    }
  };

  return (
    <div className="pt-2">
      <h3 className="text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] font-sans">{t('toolPolicy.customTools')}</h3>

      {customLoading && (
        <p className="text-[11px] text-[#666666] dark:text-[#71717a] font-sans mt-2">{t('common.loading')}</p>
      )}
      {customListError && (
        <p className="text-[11px] font-sans mt-2 text-red-700 dark:text-red-400 bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-900/50 rounded-md px-3 py-1.5">
          {customListError}
        </p>
      )}

      <form onSubmit={(e) => void handleAddCustomTool(e)} className="mt-3 p-3 rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#fafafa] dark:bg-[#17171b] space-y-2.5">
        <p className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] font-sans">{t('toolPolicy.addTool')}</p>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
          <div>
            <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1 text-[11px]">{t('toolPolicy.nameLabel')}</label>
            <input
              type="text"
              value={customName}
              onChange={(e) => setCustomName(e.target.value)}
              placeholder="my_tool"
              className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] font-mono text-xs transition"
            />
          </div>
          <div>
            <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1 text-[11px]">{t('toolPolicy.commandLabel')}</label>
            <input
              type="text"
              value={customCommand}
              onChange={(e) => setCustomCommand(e.target.value)}
              placeholder="my-script --input {args}"
              className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] font-mono text-xs transition"
            />
          </div>
        </div>
        <div>
          <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1 text-[11px]">{t('toolPolicy.descLabel')}</label>
          <input
            type="text"
            value={customDescription}
            onChange={(e) => setCustomDescription(e.target.value)}
            placeholder={t('toolPolicy.descPlaceholder')}
            className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] font-sans text-xs transition"
          />
        </div>
        <div>
          <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1 text-[11px]">{t('toolPolicy.paramsLabel')}</label>
          <textarea
            rows={3}
            value={customParamsText}
            onChange={(e) => setCustomParamsText(e.target.value)}
            placeholder={'{"type": "object", "properties": {}}'}
            spellCheck={false}
            className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] font-mono text-xs resize-y transition"
          />
        </div>
        {customFormError && (
          <p className="text-[11px] font-sans text-red-700 dark:text-red-400 bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-900/50 rounded-md px-3 py-1.5">
            {customFormError}
          </p>
        )}
        <div className="flex justify-end">
          <button
            type="submit"
            disabled={customSaving}
            className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] disabled:opacity-50 text-white font-medium transition cursor-pointer font-sans text-xs shadow-xs"
          >
            {customSaving ? t('common.saving') : t('toolPolicy.add')}
          </button>
        </div>
      </form>
    </div>
  );
}
