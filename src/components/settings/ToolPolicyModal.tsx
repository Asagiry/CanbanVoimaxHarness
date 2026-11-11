import { Fragment, useCallback, useEffect, useMemo, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import {
  useToolPolicyStore,
  AVAILABLE_TOOLS,
  getToolTranslationKey,
  isToolSchemaCustomized,
  type ExecuteShell,
} from '../../stores/toolPolicyStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import AgentIcon from '../common/AgentIcon';
import { X, ChevronDown } from 'lucide-react';
import { useTranslation, type TranslationKey } from '../../stores/languageStore';
import { useUiZoom, getZoomOverlayStyle } from '../../hooks/useUiZoom';
import ToolParamsEditor, { PolicyToggle } from './tool-policy/ToolParamsEditor';
import ToolSchemaEditor from './tool-policy/ToolSchemaEditor';
import CustomToolForm, { CustomToolRows } from './tool-policy/CustomToolForm';
import {
  CATEGORY_ORDER,
  getCategoryLabel,
  buildAgentMatrix,
  applyAgentToolToggle,
  fetchCustomToolsIpc,
  deleteCustomToolIpc,
  type CustomTool,
} from './tool-policy/toolPolicyConstants';

export default function ToolPolicyModal() {
  const { zoom } = useUiZoom();
  const {
    isModalOpen,
    closeModal,
    setStageTool,
    isToolAllowed,
    toolSettings,
    toolDescriptions,
    toolCustomSchemas,
  } = useToolPolicyStore(
    useShallow((s) => ({
      isModalOpen: s.isModalOpen,
      closeModal: s.closeModal,
      setStageTool: s.setStageTool,
      isToolAllowed: s.isToolAllowed,
      toolSettings: s.toolSettings,
      toolDescriptions: s.toolDescriptions,
      toolCustomSchemas: s.toolCustomSchemas,
    }))
  );
  const executeShell = ((toolSettings['execute_command']?.shell as string) || 'bash') as ExecuteShell;
  const { t } = useTranslation();
  const { agentPersonas, loadAgentPersonas } = useCortexStore(
    useShallow((s) => ({ agentPersonas: s.agentPersonas, loadAgentPersonas: s.loadAgentPersonas }))
  );
  const columns = useBoardStore((s) => s.columns);

  const agents = useMemo(() => buildAgentMatrix(agentPersonas, columns), [agentPersonas, columns]);

  const [expandedToolId, setExpandedToolId] = useState<string | null>(null);
  const [customTools, setCustomTools] = useState<CustomTool[]>([]);
  const [customLoading, setCustomLoading] = useState(false);
  const [customListError, setCustomListError] = useState<string | null>(null);
  const [customFormError, setCustomFormError] = useState<string | null>(null);

  const toggleExpanded = (id: string) => setExpandedToolId((prev) => (prev === id ? null : id));

  const loadCustomTools = useCallback(async () => {
    setCustomLoading(true);
    setCustomListError(null);
    try {
      setCustomTools(await fetchCustomToolsIpc());
    } catch (err) {
      setCustomTools([]);
      setCustomListError(err instanceof Error ? err.message : String(err));
    } finally {
      setCustomLoading(false);
    }
  }, []);

  useEffect(() => {
    if (isModalOpen) {
      setExpandedToolId(null);
      setCustomFormError(null);
      void loadCustomTools();
      void loadAgentPersonas();
    }
  }, [isModalOpen, loadCustomTools, loadAgentPersonas]);

  useEffect(() => {
    if (!isModalOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopImmediatePropagation();
        closeModal();
      }
    };
    window.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => window.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [isModalOpen, closeModal]);

  const handleDeleteCustomTool = async (name: string) => {
    if (!window.confirm(t('toolPolicy.deleteToolConfirm', { name }))) return;
    setCustomFormError(null);
    try {
      await deleteCustomToolIpc(name);
      await loadCustomTools();
    } catch (err) {
      setCustomFormError(err instanceof Error ? err.message : String(err));
    }
  };

  if (!isModalOpen) return null;

  return (
    <div style={getZoomOverlayStyle(zoom)} className="fixed inset-0 z-[80] flex items-center justify-center bg-black/60 dark:bg-black/80 p-3">
      <div className="w-full max-w-5xl rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] text-[#171717] dark:text-[#f4f4f5] shadow-md overflow-hidden flex flex-col max-h-[85%] transition-colors">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#ebebeb] dark:border-[#212126] px-4 py-3 bg-white dark:bg-[#111114]">
          <div>
            <h2 className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5]">{t('toolPolicy.modalTitle')}</h2>
            <p className="text-[11px] text-[#666666] dark:text-[#71717a] font-mono mt-0.5">{t('toolPolicy.modalSubtitle')}</p>
          </div>
          <button
            onClick={closeModal}
            className="p-1 rounded-md text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23] transition cursor-pointer"
            aria-label={t('common.close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body Content */}
        <div className="flex-1 min-h-0 overflow-y-auto p-4 space-y-4 font-mono text-xs">
          <div className="overflow-x-auto rounded-md border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114]">
            <table className="w-full text-left border-collapse text-xs">
              <thead>
                <tr className="border-b border-[#ebebeb] dark:border-[#212126] bg-[#f9f9f9] dark:bg-[#17171b] text-[#666666] dark:text-[#71717a] text-[10px] uppercase">
                  <th className="py-2.5 px-3 font-medium">{t('toolPolicy.toolHeader')}</th>
                  <th className="py-2.5 px-2 text-right font-medium">{t('toolPolicy.weightHeader')}</th>
                  {agents.map((agent) => (
                    <th key={agent.role} className="py-2.5 px-2 text-center font-medium border-l border-[#ebebeb] dark:border-[#212126] min-w-[95px]">
                      <div className="flex flex-col items-center gap-1">
                        <div className="flex items-center gap-1.5 justify-center max-w-[110px]">
                          <AgentIcon
                            name={agent.icon || (agent.isBuiltin ? 'Cpu' : 'Bot')}
                            className={`h-3.5 w-3.5 shrink-0 ${agent.isBuiltin ? 'text-sky-600 dark:text-sky-400' : 'text-[#171717] dark:text-[#f4f4f5]'}`}
                          />
                          <span className="truncate block font-mono font-semibold text-[#171717] dark:text-[#f4f4f5]" title={agent.role}>
                            {agent.role}
                          </span>
                        </div>
                        {agent.isBuiltin && (
                          <span className="inline-flex items-center px-1 py-0.5 rounded text-[9px] font-sans font-medium leading-none bg-sky-50 dark:bg-sky-950/60 text-sky-700 dark:text-sky-300 border border-sky-200 dark:border-sky-800">
                            {t('toolPolicy.builtin')}
                          </span>
                        )}
                      </div>
                    </th>
                  ))}
                </tr>
              </thead>

              <tbody className="divide-y divide-[#ebebeb] dark:divide-[#212126]">
                {CATEGORY_ORDER.map((catKey) => {
                  const toolsInCat = AVAILABLE_TOOLS.filter((tool) => tool.category === catKey);
                  if (toolsInCat.length === 0) return null;

                  return (
                    <Fragment key={`cat-grp-${catKey}`}>
                      <tr className="bg-[#f4f4f5]/90 dark:bg-[#17171b] border-y border-[#ebebeb] dark:border-[#212126]">
                        <td colSpan={2 + agents.length} className="py-1.5 px-3 text-[10px] font-semibold text-[#52525b] dark:text-[#71717a] uppercase tracking-wider">
                          {getCategoryLabel(catKey, t)}
                        </td>
                      </tr>
                      {toolsInCat.map((tool) => {
                        const isExpanded = expandedToolId === tool.id;
                        const isChild = Boolean(tool.parentTool);
                        const toolLabelKey = getToolTranslationKey(tool.id, 'label') as TranslationKey;
                        const toolLabel = t(toolLabelKey) !== toolLabelKey ? t(toolLabelKey) : tool.label;

                        return (
                          <Fragment key={tool.id}>
                            <tr className={`hover:bg-[#fafafa] dark:hover:bg-[#1e1e23] transition text-[#171717] dark:text-[#f4f4f5] ${isChild ? 'bg-[#fafafc] dark:bg-[#0b0b0e]' : ''}`}>
                              <td className={`py-2 px-3 align-middle ${isChild ? 'pl-7' : ''}`}>
                                <div className="flex items-start gap-1.5 text-left w-full">
                                  {isChild && (
                                    <span className="text-[#8f8f8f] dark:text-[#52525b] font-mono text-xs select-none shrink-0 mt-0.5 leading-none">
                                      {tool.id === 'subagent_list' ? '└──' : '├──'}
                                    </span>
                                  )}
                                  <button type="button" onClick={() => toggleExpanded(tool.id)} className="flex items-start gap-1.5 text-left cursor-pointer min-w-0 flex-1 group">
                                    <ChevronDown className={`w-3.5 h-3.5 shrink-0 mt-0.5 text-[#8f8f8f] dark:text-[#71717a] transition-transform ${isExpanded ? 'rotate-180' : ''}`} />
                                    <div className="min-w-0 flex-1">
                                      <div className="flex items-center gap-1.5 flex-wrap">
                                        <span className={`font-medium text-[#171717] dark:text-[#f4f4f5] leading-tight group-hover:underline underline-offset-2 ${tool.isSubagentMaster ? 'font-semibold' : ''}`}>
                                          {toolLabel}
                                        </span>
                                        {isToolSchemaCustomized(tool, toolCustomSchemas[tool.id], executeShell) ? (
                                          <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-mono leading-none bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/30">
                                            {t('toolPolicy.customSchema')}
                                          </span>
                                        ) : toolDescriptions[tool.id] !== undefined && toolDescriptions[tool.id] !== tool.description ? (
                                          <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-mono leading-none bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/30">
                                            {t('toolPolicy.customDescription')}
                                          </span>
                                        ) : null}
                                      </div>
                                      <div className="text-[10px] text-[#8f8f8f] dark:text-[#71717a] font-mono leading-tight mt-0.5">{tool.name}</div>
                                    </div>
                                  </button>
                                </div>
                              </td>

                              <td className="py-2 px-2 text-right text-[#666666] dark:text-[#71717a] tabular-nums text-[11px] align-middle">
                                {tool.tokenWeight}
                              </td>

                              {agents.map((agent) => {
                                const isMasterAllowed = isChild && tool.parentTool ? isToolAllowed(tool.parentTool, agent.role) : true;
                                const isAllowed = isToolAllowed(tool.id, agent.role);
                                const isDisabled = isChild && !isMasterAllowed;
                                const masterKey = tool.parentTool ? (getToolTranslationKey(tool.parentTool, 'label') as TranslationKey) : null;
                                const masterLabel = masterKey && t(masterKey) !== masterKey ? t(masterKey) : tool.parentTool || '';

                                return (
                                  <td key={agent.role} className="py-2 px-2 text-center border-l border-[#ebebeb] dark:border-[#212126] align-middle">
                                    <span className={`inline-flex justify-center pt-0.5 ${isDisabled ? 'opacity-40' : ''}`}>
                                      <PolicyToggle
                                        enabled={!isDisabled && isAllowed}
                                        disabled={isDisabled}
                                        onToggle={() => applyAgentToolToggle(agent.role, tool.id, !isAllowed, columns, setStageTool)}
                                        title={
                                          isDisabled
                                            ? t('toolPolicy.masterDisabledNotice', { tool: toolLabel, master: masterLabel, agent: agent.role })
                                            : t('toolPolicy.toolAgentStatus', {
                                                tool: toolLabel,
                                                agent: agent.role,
                                                status: isAllowed ? t('toolPolicy.statusEnabled') : t('toolPolicy.statusDisabled'),
                                              })
                                        }
                                      />
                                    </span>
                                  </td>
                                );
                              })}
                            </tr>
                            {isExpanded && (
                              <tr className="bg-[#f9f9fb] dark:bg-[#0b0b0e] border-b border-[#ebebeb] dark:border-[#212126]">
                                <td colSpan={2 + agents.length} className="p-3">
                                  <div className="space-y-2 w-full">
                                    <ToolParamsEditor toolId={tool.id} />
                                    <ToolSchemaEditor tool={tool} />
                                  </div>
                                </td>
                              </tr>
                            )}
                          </Fragment>
                        );
                      })}
                    </Fragment>
                  );
                })}

                <CustomToolRows
                  customTools={customTools}
                  agents={agents}
                  columns={columns}
                  expandedToolId={expandedToolId}
                  onToggleExpanded={toggleExpanded}
                  isToolAllowed={isToolAllowed}
                  setStageTool={setStageTool}
                  onDeleteCustomTool={handleDeleteCustomTool}
                />
              </tbody>
            </table>
          </div>

          <CustomToolForm
            customLoading={customLoading}
            customListError={customListError}
            customFormError={customFormError}
            setCustomFormError={setCustomFormError}
            onToolSaved={loadCustomTools}
          />
        </div>

        {/* Footer */}
        <div className="border-t border-[#ebebeb] dark:border-[#212126] bg-[#f9f9f9] dark:bg-[#17171b] px-4 py-2.5 flex items-center justify-end font-mono text-xs">
          <button
            onClick={closeModal}
            className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer shadow-xs"
          >
            {t('common.close')}
          </button>
        </div>
      </div>
    </div>
  );
}
