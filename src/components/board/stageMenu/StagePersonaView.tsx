import PersonaSelect from '../../common/PersonaSelect';
import { slugifyTitle } from '../../../lib/slug';
import { useTranslation } from '../../../stores/languageStore';

export interface AgentPersonaOption {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
  isBuiltin?: boolean;
  is_builtin?: boolean;
  icon?: string;
  color?: string;
}

export const NEW_AGENT_OPTION = '__new__';

export interface StagePersonaViewProps {
  currentPersona?: string | null;
  selectedPersona: string;
  setSelectedPersona: (val: string) => void;
  usePersonaSelect: boolean;
  personas: AgentPersonaOption[];
  personasLoading: boolean;
  personaSaving: boolean;
  isNewAgentSelected: boolean;
  setIsNewAgentSelected: (val: boolean) => void;
  newAgentName: string;
  setNewAgentName: (val: string) => void;
  personaError: string | null;
  onSave: () => void;
  onCancel: () => void;
}

export function StagePersonaView({
  currentPersona,
  selectedPersona,
  setSelectedPersona,
  usePersonaSelect,
  personas,
  personasLoading,
  personaSaving,
  isNewAgentSelected,
  setIsNewAgentSelected,
  newAgentName,
  setNewAgentName,
  personaError,
  onSave,
  onCancel,
}: StagePersonaViewProps) {
  const { t } = useTranslation();

  return (
    <div className="px-3 py-2">
      <div className="text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">{t('columnMenu.agentColumn')}</div>
      <div className="mb-1.5 text-[#8f8f8f] dark:text-[#71717a]">
        {t('columnMenu.currentAgent', { agent: currentPersona ?? t('columnMenu.notAssigned') })}
      </div>
      {usePersonaSelect ? (
        <PersonaSelect
          value={isNewAgentSelected ? NEW_AGENT_OPTION : selectedPersona}
          onChange={(val) => {
            if (val === NEW_AGENT_OPTION) {
              setIsNewAgentSelected(true);
            } else {
              setIsNewAgentSelected(false);
              setSelectedPersona(val);
            }
          }}
          disabled={personasLoading || personaSaving}
          personas={personas}
          allowNewAgent={true}
          newAgentOptionValue={NEW_AGENT_OPTION}
        />
      ) : (
        <input
          type="text"
          autoFocus
          value={selectedPersona}
          onChange={(e) => setSelectedPersona(e.target.value)}
          placeholder={personasLoading ? t('board.loadingAgents') : 'developer'}
          className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#52525b] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#17171b] font-mono transition"
        />
      )}
      {usePersonaSelect && isNewAgentSelected && (
        <>
          <input
            type="text"
            autoFocus
            value={newAgentName}
            onChange={(e) => setNewAgentName(e.target.value)}
            placeholder={t('board.newAgentNamePlaceholder')}
            className="mt-1.5 w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#52525b] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#17171b] font-mono transition"
          />
          {newAgentName.trim() !== '' ? (
            <div className="mt-1 text-[10px] font-mono text-[#8f8f8f]">
              {t('columnMenu.roleNameHint', {
                role: slugifyTitle(newAgentName) === 'column'
                  ? t('columnMenu.lettersOrDigitsRequired')
                  : slugifyTitle(newAgentName)
              })}
            </div>
          ) : (
            <div className="mt-1 text-[10px] text-[#8f8f8f]">
              {t('columnMenu.translitHint')}
            </div>
          )}
        </>
      )}
      {personaError && (
        <div className="mt-2 text-xs text-red-700 bg-red-50 border border-red-200 rounded-md px-2.5 py-1.5">
          {personaError}
        </div>
      )}
      <div className="mt-2 flex items-center justify-end space-x-2">
        <button
          type="button"
          onClick={onCancel}
          className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] transition cursor-pointer"
        >
          {t('common.back')}
        </button>
        <button
          type="button"
          disabled={personaSaving || personasLoading}
          onClick={onSave}
          className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#2c2c33]"
        >
          {t('common.save')}
        </button>
      </div>
    </div>
  );
}
