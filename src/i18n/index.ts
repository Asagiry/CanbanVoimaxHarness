import { create } from 'zustand';
import { useCallback } from 'react';
import { en } from './en';
import { ru } from './ru';
import type { Language, TranslationKey } from './types';

export * from './types';
export { en } from './en';
export { ru } from './ru';

export const translations = {
  en,
  ru,
} as const;

const STORAGE_KEY = 'cvh_language';

function getInitialLanguage(): Language {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'en' || saved === 'ru') {
      return saved;
    }
  } catch {
    // localStorage not accessible
  }
  return 'en';
}

export interface LanguageState {
  language: Language;
  setLanguage: (lang: Language) => void;
  toggleLanguage: () => void;
}

export const useLanguageStore = create<LanguageState>((set, get) => ({
  language: getInitialLanguage(),
  setLanguage: (lang: Language) => {
    try {
      localStorage.setItem(STORAGE_KEY, lang);
    } catch {
      // ignore
    }
    set({ language: lang });
  },
  toggleLanguage: () => {
    const next: Language = get().language === 'en' ? 'ru' : 'en';
    get().setLanguage(next);
  },
}));

export function interpolate(text: string, params?: Record<string, string | number>): string {
  if (!params) return text;
  let result = text;
  for (const [key, val] of Object.entries(params)) {
    result = result.replace(new RegExp(`\\{${key}\\}`, 'g'), String(val));
  }
  return result;
}

export function translate(
  lang: Language,
  key: TranslationKey,
  params?: Record<string, string | number>
): string {
  const dict = translations[lang] || translations.en;
  const raw = dict[key] ?? translations.en[key] ?? (key as string);
  return interpolate(raw, params);
}

/**
 * Direct non-reactive translation helper for callbacks / non-React utility functions
 */
export function t(key: TranslationKey, params?: Record<string, string | number>): string {
  const lang = useLanguageStore.getState().language;
  return translate(lang, key, params);
}

/**
 * Direct getter for the current language
 */
export function currentLanguage(): Language {
  return useLanguageStore.getState().language;
}

/**
 * Direct setter for the current language
 */
export function setLanguage(lang: Language): void {
  useLanguageStore.getState().setLanguage(lang);
}

/**
 * React Hook for component localization with instantaneous reactivity
 */
export function useTranslation() {
  const language = useLanguageStore((s) => s.language);
  const setLang = useLanguageStore((s) => s.setLanguage);
  const toggleLanguage = useLanguageStore((s) => s.toggleLanguage);

  const tBound = useCallback(
    (key: TranslationKey, params?: Record<string, string | number>): string => {
      return translate(language, key, params);
    },
    [language]
  );

  return {
    language,
    setLanguage: setLang,
    toggleLanguage,
    t: tBound,
  };
}
