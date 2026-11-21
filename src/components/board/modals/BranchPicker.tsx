import React, { useState, useEffect, useMemo, useRef } from 'react';
import { ChevronDown, GitBranch, Pin, Search, X } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';
import { useProjectStore } from '../../../stores/projectStore';

export interface BranchPickerProps {
  branches: string[];
  value: string;
  onChange: (branch: string) => void;
}

const STORAGE_PREFIX = 'cvh_pinned_branches:';

function loadPinnedBranches(projectPath?: string): string[] {
  try {
    const raw = localStorage.getItem(`${STORAGE_PREFIX}${projectPath || 'default'}`);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

function savePinnedBranches(projectPath: string | undefined, pinned: string[]): void {
  try {
    localStorage.setItem(`${STORAGE_PREFIX}${projectPath || 'default'}`, JSON.stringify(pinned));
  } catch {
    // Ignore storage quota errors
  }
}

export default function BranchPicker({ branches, value, onChange }: BranchPickerProps) {
  const { t } = useTranslation();
  const projectPath = useProjectStore((s) => s.activeProject?.rootPath);

  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [pinned, setPinned] = useState<string[]>(() => loadPinnedBranches(projectPath));

  const containerRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setPinned(loadPinnedBranches(projectPath));
  }, [projectPath]);

  useEffect(() => {
    if (!isOpen) return;
    setQuery('');
    const raf = requestAnimationFrame(() => searchInputRef.current?.focus());

    const handleOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleOutside);
    window.addEventListener('keydown', handleEsc, { capture: true });
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener('mousedown', handleOutside);
      window.removeEventListener('keydown', handleEsc, { capture: true });
    };
  }, [isOpen]);

  const togglePin = (branch: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setPinned((prev) => {
      const next = prev.includes(branch) ? prev.filter((b) => b !== branch) : [branch, ...prev];
      savePinnedBranches(projectPath, next);
      return next;
    });
  };

  const { pinnedMatches, otherMatches } = useMemo(() => {
    const q = query.trim().toLowerCase();
    const pinnedSet = new Set(pinned);
    const filtered = q ? branches.filter((b) => b.toLowerCase().includes(q)) : branches;
    return {
      pinnedMatches: filtered.filter((b) => pinnedSet.has(b)),
      otherMatches: filtered.filter((b) => !pinnedSet.has(b)),
    };
  }, [branches, pinned, query]);

  const handleSelect = (branch: string) => {
    onChange(branch);
    setIsOpen(false);
  };

  const renderBranchRow = (branch: string, isPinned: boolean) => {
    const isSelected = value === branch;
    return (
      <div
        key={branch}
        onClick={() => handleSelect(branch)}
        className={`flex items-center justify-between gap-2 px-2.5 py-1.5 text-xs font-mono cursor-pointer transition ${
          isSelected
            ? 'bg-[#f4f4f5] dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] font-medium'
            : 'text-[#171717] dark:text-[#e4e4e7] hover:bg-[#fafafa] dark:hover:bg-[#1c1c21]'
        }`}
      >
        <span className="truncate">{branch}</span>
        <button
          type="button"
          onClick={(e) => togglePin(branch, e)}
          title={isPinned ? t('newTask.unpinBranch') : t('newTask.pinBranch')}
          aria-label={isPinned ? t('newTask.unpinBranch') : t('newTask.pinBranch')}
          className={`p-0.5 rounded transition shrink-0 cursor-pointer ${
            isPinned
              ? 'text-amber-500 hover:text-amber-600'
              : 'text-[#a1a1aa] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
          }`}
        >
          <Pin className={`h-3 w-3 ${isPinned ? 'fill-current' : ''}`} />
        </button>
      </div>
    );
  };

  return (
    <div ref={containerRef} className="relative">
      <div
        role="button"
        tabIndex={0}
        onClick={() => setIsOpen((o) => !o)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            setIsOpen((o) => !o);
          }
        }}
        className="w-full flex items-center justify-between gap-1.5 px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] hover:border-[#d4d4d8] dark:hover:border-[#3f3f46] text-xs cursor-pointer transition"
      >
        <span className="flex items-center gap-1.5 min-w-0">
          <GitBranch className="h-3.5 w-3.5 text-[#71717a] shrink-0" />
          <span className={`truncate ${value ? 'font-mono' : 'font-sans text-[#71717a]'}`}>
            {value || t('newTask.baseBranchDefault')}
          </span>
        </span>
        <span className="flex items-center gap-1 shrink-0">
          {value && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onChange('');
              }}
              aria-label={t('common.close')}
              className="p-0.5 rounded text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] cursor-pointer"
            >
              <X className="h-3 w-3" />
            </button>
          )}
          <ChevronDown className={`h-3.5 w-3.5 text-[#71717a] transition-transform ${isOpen ? 'rotate-180' : ''}`} />
        </span>
      </div>

      {isOpen && (
        <div className="absolute left-0 right-0 bottom-full mb-1 z-50 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] shadow-lg overflow-hidden">
          <div className="flex items-center gap-1.5 px-2.5 py-1.5 border-b border-[#ebebeb] dark:border-[#212126]">
            <Search className="h-3.5 w-3.5 text-[#71717a] shrink-0" />
            <input
              ref={searchInputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  const first = pinnedMatches[0] ?? otherMatches[0];
                  if (first) handleSelect(first);
                }
              }}
              placeholder={t('newTask.baseBranchSearch')}
              className="w-full bg-transparent text-xs font-mono text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden"
            />
          </div>

          <div className="max-h-52 overflow-y-auto divide-y divide-[#f4f4f5] dark:divide-[#1c1c21]">
            <div
              onClick={() => handleSelect('')}
              className={`px-2.5 py-1.5 text-xs font-sans cursor-pointer transition ${
                !value
                  ? 'bg-[#f4f4f5] dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] font-medium'
                  : 'text-[#71717a] hover:bg-[#fafafa] dark:hover:bg-[#1c1c21]'
              }`}
            >
              {t('newTask.baseBranchDefault')}
            </div>

            {pinnedMatches.length > 0 && (
              <div>
                <div className="px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider text-[#8f8f8f] dark:text-[#71717a] bg-[#fafafa] dark:bg-[#17171b]">
                  {t('newTask.baseBranchPinned')}
                </div>
                {pinnedMatches.map((b) => renderBranchRow(b, true))}
              </div>
            )}

            {otherMatches.length > 0 && (
              <div>
                {pinnedMatches.length > 0 && (
                  <div className="px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider text-[#8f8f8f] dark:text-[#71717a] bg-[#fafafa] dark:bg-[#17171b]">
                    {t('newTask.baseBranchAll')}
                  </div>
                )}
                {otherMatches.map((b) => renderBranchRow(b, false))}
              </div>
            )}

            {pinnedMatches.length === 0 && otherMatches.length === 0 && (
              <div className="px-2.5 py-3 text-center text-xs text-[#8f8f8f] dark:text-[#71717a]">
                {t('newTask.baseBranchEmpty')}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
