import { Settings } from 'lucide-react';
import type { PipelineColumn, CortexAgentStage } from '../../../types';
import type { AgentPersonaInfo } from '../../../stores/cortexStore';
import { getDefaultColumnIcon } from '../../../lib/columnIcons';
import {
  BACKLOG_COLOR,
  READY_COLOR,
  isAllowedAgentColor,
  getDeterministicAgentColor,
} from '../../../lib/agentPalette';
import type { SharedFolderState } from '../types';
import StageHeaderButton from '../tree/StageHeaderButton';
import NewCortexFileForm from '../forms/NewCortexFileForm';
import AgentField from './AgentField';
import HooksFolderBlock from '../tree/HooksFolderBlock';
import FileTreeItem from '../tree/FileTreeItem';
import SettingMissingBlock from '../tree/SettingMissingBlock';

export interface ColumnStageCardProps {
  stage: CortexAgentStage;
  isBacklogSystem: boolean;
  column: PipelineColumn | undefined;
  personas: AgentPersonaInfo[];
  agentSaving: boolean;
  agentSaveError: string | null;
  onAgentChange: (columnId: string, value: string) => void;
  shared: SharedFolderState;
  activeFileId: string | null;
  fileMissing: Record<string, boolean>;
  isFocused: boolean;
  isSearching: boolean;
  onToggleStage: () => void;
  onToggleHooks: () => void;
  onOpenFile: (fileId: string) => void;
  onCreateSetting: (fileId: string) => void;
  onOpenCreateForm: (kind: 'rule' | 'hook') => void;
  onCloseCreate: () => void;
}

export function ColumnStageCard({
  stage,
  isBacklogSystem,
  column,
  personas,
  agentSaving,
  agentSaveError,
  onAgentChange,
  shared,
  activeFileId,
  fileMissing,
  isFocused,
  isSearching,
  onToggleStage,
  onToggleHooks,
  onOpenFile,
  onCreateSetting,
  onOpenCreateForm,
  onCloseCreate,
}: ColumnStageCardProps) {
  const hookFiles = stage.files.filter((f) => f.type === 'hook');
  const settingsFile = stage.files.find((f) => f.type === 'settings');
  const areHooksOpen = shared.openHooksFolders[stage.stageId] ?? true;
  const settingsIsMissing = settingsFile ? !!fileMissing[settingsFile.id] : false;

  const isBacklog = stage.stageId === 'backlog';
  const isReady = stage.stageId === 'ready';
  const assignedPersona = column?.persona ? personas.find((p) => p.role === column.persona) : undefined;
  const effectiveIcon = isBacklog
    ? (column?.icon || stage.icon || getDefaultColumnIcon('backlog'))
    : isReady
    ? (column?.icon || stage.icon || getDefaultColumnIcon('ready'))
    : (assignedPersona?.icon || column?.icon || stage.icon || getDefaultColumnIcon(stage.stageId));
  const effectiveColor = isBacklog
    ? BACKLOG_COLOR
    : isReady
    ? READY_COLOR
    : (assignedPersona?.color || (column?.color && isAllowedAgentColor(column.color) ? column.color : (column?.persona ? getDeterministicAgentColor(column.persona) : stage.color)));

  return (
    <div
      id={`cortex-stage-${stage.stageId}`}
      className={`rounded-md transition-colors ${
        isFocused
          ? 'bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shadow-xs'
          : 'hover:bg-[#f4f4f5] dark:hover:bg-[#161b22]'
      }`}
    >
      <StageHeaderButton
        stage={stage}
        hint={stage.personaName}
        overrideIcon={effectiveIcon}
        overrideColor={effectiveColor}
        onToggle={onToggleStage}
      />

      {/* Инлайн-форма создания файла новой схемы */}
      {shared.creatingStageId === stage.stageId && (
        <NewCortexFileForm
          stage={stage}
          initialKind={shared.creatingKind}
          fixedKind={isBacklogSystem ? undefined : 'hook'}
          onClose={onCloseCreate}
        />
      )}

      {/* Children */}
      {stage.isExpanded && (
        <div className="pl-2.5 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-2 my-0.5">
          {!isBacklogSystem && !isReady && !column?.terminal && column && (
            <AgentField
              columnId={column.id}
              current={column.persona}
              personas={personas}
              saving={agentSaving}
              error={agentSaveError}
              onChange={onAgentChange}
            />
          )}
          <HooksFolderBlock
            files={hookFiles}
            isOpen={areHooksOpen}
            isSearching={isSearching}
            activeFileId={activeFileId}
            onToggle={onToggleHooks}
            onNew={() => onOpenCreateForm('hook')}
            onOpen={onOpenFile}
          />

          {isBacklogSystem && settingsFile && !settingsIsMissing && (
            <FileTreeItem
              file={settingsFile}
              displayName="setting"
              isActive={activeFileId === settingsFile.id}
              icon={<Settings className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
              onSelect={() => onOpenFile(settingsFile.id)}
            />
          )}

          {isBacklogSystem && settingsFile && settingsIsMissing && (
            <SettingMissingBlock onCreate={() => onCreateSetting(settingsFile.id)} />
          )}
        </div>
      )}
    </div>
  );
}

export default ColumnStageCard;
