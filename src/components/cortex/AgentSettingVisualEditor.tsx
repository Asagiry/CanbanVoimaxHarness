import { useState, useEffect, useMemo, useRef } from 'react';
import { useTranslation } from '../../stores/languageStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useCortexStore } from '../../stores/cortexStore';
import { AlertTriangle, Code2 } from 'lucide-react';
import type { ReasoningEffort } from '../../types';
import {
  getDeterministicAgentColor,
  getDeterministicAgentIcon,
  isAllowedAgentColor,
} from '../../lib/agentPalette';
import {
  parseSettingJson,
  serializeSetting,
  buildDynamicModelOptions,
  type ParsedSetting,
  CustomizationSection,
  ModelParametersSection,
  ContextLimitsSection,
} from './settingEditor';

export * from './settingEditor';

export interface AgentSettingVisualEditorProps {
  content: string;
  role?: string;
  onChange: (newContent: string) => void;
  onSwitchToCode?: () => void;
}

export default function AgentSettingVisualEditor({
  content,
  role,
  onChange,
  onSwitchToCode,
}: AgentSettingVisualEditorProps) {
  const { t } = useTranslation();
  const providers = useSettingsStore((s) => s.providers);
  const settingsModel = useSettingsStore((s) => s.settings.model);
  const agentPersonas = useCortexStore((s) => s.agentPersonas);

  const assignedPersona = useMemo(() => {
    return role ? agentPersonas.find((p) => p.role === role) : undefined;
  }, [role, agentPersonas]);

  const resolveFallbackIcon = (parsedIcon?: string) => {
    return parsedIcon || assignedPersona?.icon || (role ? getDeterministicAgentIcon(role) : 'Bot');
  };

  const resolveFallbackColor = (parsedColor?: string) => {
    if (parsedColor && isAllowedAgentColor(parsedColor)) return parsedColor;
    if (assignedPersona?.color && isAllowedAgentColor(assignedPersona.color)) return assignedPersona.color;
    return role ? getDeterministicAgentColor(role) : '#0ea5e9';
  };

  const lastEmittedRef = useRef<string | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);

  const [model, setModel] = useState<string>('inherit');
  const [reasoningEffort, setReasoningEffort] = useState<ReasoningEffort>('off');
  const [contextWindow, setContextWindow] = useState<number>(200000);
  const [maxTokens, setMaxTokens] = useState<number>(8192);
  const [allowedToolsData, setAllowedToolsData] = useState<{
    has: boolean;
    key?: string;
    value?: unknown;
  }>({ has: false });
  const [icon, setIcon] = useState<string>(() => {
    const { parsed } = parseSettingJson(content);
    return resolveFallbackIcon(parsed.icon);
  });
  const [color, setColor] = useState<string>(() => {
    const { parsed } = parseSettingJson(content);
    return resolveFallbackColor(parsed.color);
  });
  const [rawExtra, setRawExtra] = useState<Record<string, unknown>>({});

  useEffect(() => {
    if (content === lastEmittedRef.current) return;

    const { parsed, error } = parseSettingJson(content);
    if (error) {
      setParseError(error);
      return;
    }

    setParseError(null);
    setModel(parsed.model ?? 'inherit');
    setReasoningEffort(parsed.reasoningEffort);
    setContextWindow(parsed.contextWindow);
    setMaxTokens(parsed.maxTokens);
    setAllowedToolsData({
      has: parsed.hasAllowedTools,
      key: parsed.allowedToolsKey,
      value: parsed.allowedTools,
    });
    setIcon(resolveFallbackIcon(parsed.icon));
    setColor(resolveFallbackColor(parsed.color));
    setRawExtra(parsed.rawExtra);

    lastEmittedRef.current = content;
  }, [content, assignedPersona, role]);

  const emitChanges = (
    nextModel: string,
    nextReasoning: ReasoningEffort,
    nextCw: number,
    nextMt: number,
    nextIcon: string | undefined,
    nextColor: string | undefined,
    nextExtra: Record<string, unknown>
  ) => {
    const nextSetting: ParsedSetting = {
      model: nextModel,
      reasoningEffort: nextReasoning,
      contextWindow: nextCw,
      maxTokens: nextMt,
      hasAllowedTools: allowedToolsData.has,
      allowedToolsKey: allowedToolsData.key,
      allowedTools: allowedToolsData.value,
      icon: nextIcon,
      color: nextColor,
      rawExtra: nextExtra,
    };
    const serialized = serializeSetting(nextSetting);
    lastEmittedRef.current = serialized;
    onChange(serialized);
  };

  const handleModelChange = (newModel: string) => {
    setModel(newModel);
    emitChanges(newModel, reasoningEffort, contextWindow, maxTokens, icon, color, rawExtra);
  };

  const handleReasoningEffortChange = (newEffort: ReasoningEffort) => {
    setReasoningEffort(newEffort);
    emitChanges(model, newEffort, contextWindow, maxTokens, icon, color, rawExtra);
  };

  const handleContextWindowChange = (newCw: number) => {
    const val = Math.max(1000, newCw);
    setContextWindow(val);
    emitChanges(model, reasoningEffort, val, maxTokens, icon, color, rawExtra);
  };

  const handleMaxTokensChange = (newMt: number) => {
    const val = Math.max(256, newMt);
    setMaxTokens(val);
    emitChanges(model, reasoningEffort, contextWindow, val, icon, color, rawExtra);
  };

  const handleIconChange = (newIcon: string) => {
    setIcon(newIcon);
    emitChanges(model, reasoningEffort, contextWindow, maxTokens, newIcon, color, rawExtra);
  };

  const handleColorChange = (newColor: string) => {
    if (!isAllowedAgentColor(newColor)) return;
    setColor(newColor);
    emitChanges(model, reasoningEffort, contextWindow, maxTokens, icon, newColor, rawExtra);
  };

  const dynamicModelOptions = useMemo(
    () => buildDynamicModelOptions(providers, settingsModel),
    [providers, settingsModel],
  );
  const dynamicModels = useMemo(
    () => dynamicModelOptions.map((option) => option.id),
    [dynamicModelOptions],
  );

  if (parseError) {
    return (
      <div className="p-6 space-y-4 max-w-xl mx-auto my-auto text-center font-sans">
        <div className="inline-flex p-3 rounded-full bg-amber-500/10 text-amber-500">
          <AlertTriangle className="h-8 w-8" />
        </div>
        <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5]">
          {t('ruleEditor.jsonParseError')}
        </h3>
        <p className="text-xs text-[#666666] dark:text-[#71717a] font-mono bg-[#f4f4f5] dark:bg-[#17171b] p-3 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] text-left break-all">
          {parseError}
        </p>
        {onSwitchToCode && (
          <button
            type="button"
            onClick={onSwitchToCode}
            className="inline-flex items-center space-x-2 px-4 py-2 rounded-md bg-[#171717] hover:bg-[#262626] dark:bg-[#238636] dark:hover:bg-[#2ea043] text-white text-xs font-mono font-medium transition cursor-pointer"
          >
            <Code2 className="h-4 w-4" />
            <span>{t('ruleEditor.switchToCode')}</span>
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="w-full h-full overflow-y-auto px-5 py-4 space-y-5 font-sans select-none text-[#171717] dark:text-[#f4f4f5]">
      <CustomizationSection
        role={role}
        icon={icon}
        color={color}
        onIconChange={handleIconChange}
        onColorChange={handleColorChange}
      />
      <ModelParametersSection
        model={model}
        reasoningEffort={reasoningEffort}
        dynamicModelOptions={dynamicModelOptions}
        dynamicModels={dynamicModels}
        onModelChange={handleModelChange}
        onReasoningEffortChange={handleReasoningEffortChange}
      />
      <ContextLimitsSection
        contextWindow={contextWindow}
        maxTokens={maxTokens}
        onContextWindowChange={handleContextWindowChange}
        onMaxTokensChange={handleMaxTokensChange}
      />
    </div>
  );
}
