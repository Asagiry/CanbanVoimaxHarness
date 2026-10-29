import type { en } from './en';

export type Language = 'en' | 'ru';

export type TranslationKey = keyof typeof en;

export type TranslationDict = Record<TranslationKey, string>;
