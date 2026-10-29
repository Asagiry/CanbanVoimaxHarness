import { Settings, User, Cpu, Plus } from 'lucide-react';
import type { CortexAgentStage } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { getDeterministicAgentColor } from '../../../lib/agentPalette';
import type { SharedFolderState } from '../types';
import StageHeaderButton from '../tree/StageHeaderButton';
import NewCortexFileForm from '../forms/NewCortexFileForm';
import FileTreeItem from '../tree/FileTreeItem';
import RulesFolderBlock from '../tree/RulesFolderBlock';
import SettingMissingBlock from '../tree/SettingMissingBlock';

export interface AgentStageCardProps {
  stage: CortexAgentStage;
  shared: SharedFolderState;
  activeFileId: string | null;
  fileMissing: Record<string, boolean>;
  isFocused: boolean;
  isSearching: boolean;
  onToggleStage: () => void;
  onToggleRules: () => void;
  onOpenFile: (fileId: string) => void;
  onCreatePersona: (fileId: string) => void;
  onCreateSetting: (fileId: string) => void;
  onOpenCreateForm: (kind: 'rule') => void;
  onPlusHeader: () => void;
  onCloseCreate: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}

export function AgentStageCard({
  stage,
  shared,
  activeFileId,
  fileMissing,
  isFocused,
  isSearching,
  onToggleStage,
  onToggleRules,
  onOpenFile,
  onCreatePersona,
  onCreateSetting,
  onOpenCreateForm,
  onPlusHeader,
  onCloseCreate,
  onContextMenu,
}: AgentStageCardProps) {
  const { t } = useTranslation();
  const personaFile = stage.files.find((f) => f.type === 'persona');
  const ruleFiles = stage.files.filter((f) => f.type === 'rule');
  const settingsFile = stage.files.find((f) => f.type === 'settings');
  const areRulesOpen = shared.openRulesFolders[stage.stageId] ?? true;
  const personaIsMissing = personaFile ? !!fileMissing[personaFile.id] : false;
  const settingsIsMissing = settingsFile ? !!fileMissing[settingsFile.id] : false;
  const isBuiltin = stage.personaName === 'cvh-dev' || Boolean(stage.isBuiltin);

  return (
    <div
      id={`cortex-agent-${stage.personaName}`}
      onContextMenu={onContextMenu}
      className={`rounded-md transition-colors ${
        isFocused
          ? 'bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shadow-xs'
          : 'hover:bg-[#f4f4f5] dark:hover:bg-[#161b22]'
      }`}
    >
      <StageHeaderButton
        stage={stage}
        hint=""
        overrideIcon={stage.icon}
        overrideColor={stage.color || getDeterministicAgentColor(stage.personaName)}
        onToggle={onToggleStage}
        onPlus={onPlusHeader}
      />

      {/* Инлайн-форма создания файла новой схемы (у агентов только правила) */}
      {shared.creatingStageId === stage.stageId && (
        <NewCortexFileForm
          stage={stage}
          initialKind="rule"
          fixedKind="rule"
          onClose={onCloseCreate}
        />
      )}

      {/* Children */}
      {stage.isExpanded && (
        <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-3 my-0.5">
          {/* 1. Персона: .cvh/agents/<role>/persona.md */}
          {personaFile && !personaIsMissing && (
            <FileTreeItem
              file={personaFile}
              displayName={personaFile.name}
              isActive={activeFileId === personaFile.id}
              icon={
                isBuiltin ? (
                  <Cpu className="h-3.5 w-3.5 text-indigo-600 shrink-0" />
                ) : (
                  <User className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                )
              }
              onSelect={() => onOpenFile(personaFile.id)}
            />
          )}

          {/* Персона отсутствует на диске: плейсхолдер + создание шаблона */}
          {personaFile && personaIsMissing && (
            <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] space-y-1">
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
                {t('cortex.personaNotConfigured')}
              </div>
              <button
                onClick={() => onCreatePersona(personaFile.id)}
                className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white dark:text-[#f0f6fc] transition cursor-pointer"
                title={t('cortex.createPersonaTemplate')}
              >
                <Plus className="h-3 w-3 shrink-0" />
                <span>{t('cortex.createTemplate')}</span>
              </button>
            </div>
          )}

          {/* 2. Правила: .cvh/agents/<role>/rules/ */}
          <RulesFolderBlock
            files={ruleFiles}
            isOpen={areRulesOpen}
            isSearching={isSearching}
            activeFileId={activeFileId}
            onToggle={onToggleRules}
            onNew={() => onOpenCreateForm('rule')}
            onOpen={onOpenFile}
          />

          {/* 3. Настройки: .cvh/agents/<role>/setting */}
          {settingsFile && !settingsIsMissing && (
            <FileTreeItem
              file={settingsFile}
              displayName="setting"
              isActive={activeFileId === settingsFile.id}
              icon={<Settings className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
              onSelect={() => onOpenFile(settingsFile.id)}
            />
          )}

          {settingsFile && settingsIsMissing && (
            <SettingMissingBlock onCreate={() => onCreateSetting(settingsFile.id)} />
          )}
        </div>
      )}
    </div>
  );
}

export default AgentStageCard;
