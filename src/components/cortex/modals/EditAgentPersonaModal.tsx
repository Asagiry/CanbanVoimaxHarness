import { useState, useEffect } from 'react';
import { Palette, X, Check } from 'lucide-react';
import AgentIcon from '../../common/AgentIcon';
import { useTranslation } from '../../../stores/languageStore';
import {
  AGENT_PALETTE,
  AGENT_ICONS,
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../../lib/agentPalette';
import type { AgentPersonaInfo } from '../../../stores/cortexStore';

export interface EditAgentPersonaModalProps {
  isOpen: boolean;
  persona: AgentPersonaInfo | null;
  onClose: () => void;
  onRename: (oldRole: string, newRole: string) => Promise<void>;
  onColorChange: (role: string, color: string) => Promise<void>;
  onIconChange: (role: string, icon: string) => Promise<void>;
}

export function EditAgentPersonaModal({
  isOpen,
  persona,
  onClose,
  onRename,
  onColorChange,
  onIconChange,
}: EditAgentPersonaModalProps) {
  const role = persona?.role ?? '';
  const isBuiltin = role === 'cvh-dev' || Boolean(persona?.isBuiltin);
  const [newName, setNewName] = useState(role);
  const [selectedColor, setSelectedColor] = useState(
    persona?.color && isAllowedAgentColor(persona.color)
      ? persona.color
      : getDeterministicAgentColor(role)
  );
  const [selectedIcon, setSelectedIcon] = useState(
    persona?.icon || (isBuiltin ? 'Cpu' : 'Bot')
  );
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { t } = useTranslation();

  useEffect(() => {
    if (isOpen && persona) {
      setNewName(persona.role);
      setSelectedColor(
        persona.color && isAllowedAgentColor(persona.color)
          ? persona.color
          : getDeterministicAgentColor(persona.role)
      );
      setSelectedIcon(persona.icon || (persona.isBuiltin ? 'Cpu' : 'Bot'));
      setError(null);
      setIsSubmitting(false);
    }
  }, [isOpen, persona]);

  if (!isOpen || !persona) return null;

  const handleSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const trimmed = newName.trim().toLowerCase();
    if (!trimmed) {
      setError(t('cortex.errEmptyRole'));
      return;
    }
    if (!/^[a-z0-9][a-z0-9-]*$/.test(trimmed)) {
      setError(t('cortex.errRolePattern'));
      return;
    }

    if (!isAllowedAgentColor(selectedColor)) {
      setError(t('cortex.errColorReserved'));
      return;
    }

    setIsSubmitting(true);
    setError(null);
    try {
      let currentRole = role;
      if (trimmed !== role && !isBuiltin) {
        await onRename(role, trimmed);
        currentRole = trimmed;
      }
      if (selectedColor !== (persona.color ?? '')) {
        await onColorChange(currentRole, selectedColor);
      }
      if (selectedIcon !== (persona.icon ?? '')) {
        await onIconChange(currentRole, selectedIcon);
      }
      onClose();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-4 animate-in fade-in duration-150"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-lg border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] p-5 shadow-md space-y-4 max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-[#f4f4f5] dark:border-[#21262d] pb-3">
          <div className="flex items-center space-x-2">
            <Palette className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
            <h3 className="font-semibold text-sm text-[#171717] dark:text-[#f0f6fc]">{t('cortex.configAgentTitle', { role })}</h3>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-md text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Имя роли */}
          <div>
            <label className="block text-xs font-medium text-[#52525b] dark:text-[#8b949e] mb-1">
              {t('cortex.roleNameLabel')}
            </label>
            <input
              type="text"
              disabled={isBuiltin}
              value={newName}
              onChange={(e) => {
                setNewName(e.target.value);
                setError(null);
              }}
              placeholder={t('cortex.roleNamePlaceholder')}
              className="w-full px-3 py-1.5 rounded-lg border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] text-xs font-mono focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:outline-hidden transition disabled:opacity-50 disabled:bg-[#f4f4f5] dark:disabled:bg-[#0d1117] disabled:text-[#8f8f8f] dark:disabled:text-[#6e7681]"
            />
            {isBuiltin && (
              <p className="mt-1 text-[11px] text-[#71717a] dark:text-[#8b949e]">
                {t('cortex.builtinCannotRename')}
              </p>
            )}
          </div>

          {/* Палитра цветов агента */}
          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="block text-xs font-medium text-[#52525b] dark:text-[#8b949e]">
                {t('cortex.agentColorLabel')}
              </label>
              <span className="text-[10px] font-mono text-[#71717a] dark:text-[#8b949e]">
                {selectedColor}
              </span>
            </div>
            <div className="p-2.5 rounded-lg border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] space-y-2">
              <div className="flex flex-wrap gap-2 items-center">
                {AGENT_PALETTE.map((c) => {
                  const isSelected = selectedColor.toLowerCase() === c.toLowerCase();
                  return (
                    <button
                      key={c}
                      type="button"
                      onClick={() => setSelectedColor(c)}
                      aria-label={c}
                      className={`h-7 w-7 rounded-full transition-transform cursor-pointer flex items-center justify-center shadow-xs ${
                        isSelected ? 'scale-110 ring-2 ring-offset-2 ring-[#171717] dark:ring-[#f0f6fc]' : 'hover:scale-105'
                      }`}
                      style={{ backgroundColor: c }}
                    >
                      {isSelected && <Check className="h-3.5 w-3.5 text-white drop-shadow" />}
                    </button>
                  );
                })}
              </div>
              <p className="text-[10px] font-sans text-[#71717a] dark:text-[#8b949e] leading-tight">
                {t('cortex.colorNotice')}
              </p>
            </div>
          </div>

          {/* Выбор SVG иконки */}
          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="block text-xs font-medium text-[#52525b] dark:text-[#8b949e]">
                {t('cortex.agentIconLabel')}
              </label>
              <span className="text-[10px] font-mono text-[#71717a] dark:text-[#8b949e]">
                [{selectedIcon}]
              </span>
            </div>
            <div className="p-2 rounded-lg border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22]">
              <div className="grid grid-cols-6 gap-1.5 max-h-36 overflow-y-auto p-1">
                {AGENT_ICONS.map((ic) => {
                  const isSelected = selectedIcon === ic;
                  return (
                    <button
                      key={ic}
                      type="button"
                      onClick={() => setSelectedIcon(ic)}
                      aria-label={ic}
                      className={`flex flex-col items-center justify-center p-2 rounded-md transition cursor-pointer ${
                        isSelected
                          ? 'bg-white dark:bg-[#21262d] border-2 border-[#171717] dark:border-[#f0f6fc] shadow-xs'
                          : 'hover:bg-white/80 dark:hover:bg-[#21262d]/80 border border-transparent text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                      }`}
                    >
                      <AgentIcon
                        name={ic}
                        className="h-4 w-4"
                        style={{ color: isSelected ? selectedColor : undefined }}
                      />
                    </button>
                  );
                })}
              </div>
            </div>
          </div>

          {error && (
            <div className="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 p-2 rounded-md border border-red-200 dark:border-red-900/50">
              {error}
            </div>
          )}

          <div className="flex items-center justify-end space-x-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d]">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-3 py-1.5 rounded-md text-xs font-medium text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="px-3.5 py-1.5 rounded-md text-xs font-medium bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white transition cursor-pointer disabled:opacity-50 flex items-center space-x-1.5"
            >
              {isSubmitting ? <span>{t('common.saving')}</span> : <span>{t('common.save')}</span>}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

export default EditAgentPersonaModal;
