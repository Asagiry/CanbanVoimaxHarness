import { useState, useEffect } from 'react';
import PersonaSelect from '../../common/PersonaSelect';
import { useTranslation } from '../../../stores/languageStore';
import type { AgentPersonaInfo } from '../../../stores/cortexStore';

export interface AgentFieldProps {
  columnId: string;
  current: string | null;
  personas: AgentPersonaInfo[];
  saving: boolean;
  error: string | null;
  onChange: (columnId: string, value: string) => void;
}

export function AgentField({
  columnId,
  current,
  personas,
  saving,
  error,
  onChange,
}: AgentFieldProps) {
  const { t } = useTranslation();
  const currentValue = current ?? '';
  const [draft, setDraft] = useState(currentValue);

  useEffect(() => {
    setDraft(current ?? '');
  }, [current]);

  return (
    <div className="px-0.5 py-0.5 min-w-0 w-full">
      <div className="flex items-center gap-1.5 min-w-0 w-full">
        <span className="text-[10px] font-sans font-semibold uppercase tracking-wide text-[#8f8f8f] shrink-0">
          {t('cortex.agentField')}
        </span>
        {personas.length > 0 ? (
          <PersonaSelect
            value={currentValue}
            disabled={saving}
            onChange={(role) => onChange(columnId, role)}
            allowEmpty={true}
            emptyLabel={t('cortex.noAgent')}
            personas={personas}
            compact={true}
            className="flex-1 min-w-0"
          />
        ) : (
          <input
            type="text"
            value={draft}
            disabled={saving}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => {
              if (draft !== currentValue) onChange(columnId, draft);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
              if (e.key === 'Escape') {
                setDraft(currentValue);
                (e.target as HTMLInputElement).blur();
              }
            }}
            placeholder="developer"
            title={t('cortex.columnAgent')}
            className="flex-1 min-w-0 px-1.5 py-0.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono text-[11px] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] transition disabled:opacity-50"
          />
        )}
        {saving && (
          <span className="text-[10px] font-mono text-[#8f8f8f] shrink-0">...</span>
        )}
      </div>
      {error && (
        <div className="mt-1 text-[11px] font-sans text-red-600">{error}</div>
      )}
    </div>
  );
}

export default AgentField;
