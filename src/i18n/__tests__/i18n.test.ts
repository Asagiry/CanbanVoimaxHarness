import { describe, it, expect, beforeEach } from 'vitest';
import {
  t,
  currentLanguage,
  setLanguage,
  translations,
  interpolate,
  translate,
  useLanguageStore,
  type TranslationKey,
} from '../index';
import { en } from '../en';
import { ru } from '../ru';

describe('i18n SRP decomposition', () => {
  beforeEach(() => {
    setLanguage('en');
  });

  it('exports translations dictionary with en and ru', () => {
    expect(translations.en).toBeDefined();
    expect(translations.ru).toBeDefined();
    expect(translations.en).toBe(en);
    expect(translations.ru).toBe(ru);
  });

  it('preserves exact key parity between en and ru', () => {
    const enKeys = Object.keys(en);
    const ruKeys = Object.keys(ru);

    expect(enKeys.length).toBe(544);
    expect(ruKeys.length).toBe(544);

    const missingInRu = enKeys.filter((k) => !(k in ru));
    const missingInEn = ruKeys.filter((k) => !(k in en));

    expect(missingInRu).toEqual([]);
    expect(missingInEn).toEqual([]);
  });

  it('correctly returns translations via t() and supports language switching', () => {
    setLanguage('en');
    expect(currentLanguage()).toBe('en');
    const testKey: TranslationKey = 'common.cancel';
    expect(t(testKey)).toBe('Cancel');
    expect(t('common.save')).toBe('Save');

    setLanguage('ru');
    expect(currentLanguage()).toBe('ru');
    expect(t('common.cancel')).toBe('Отмена');
    expect(t('common.save')).toBe('Сохранить');
  });

  it('correctly interpolates parameters in translations', () => {
    setLanguage('en');
    expect(t('cortex.agentContextMenu', { role: 'tester' })).toBe('Agent: tester');

    setLanguage('ru');
    expect(t('cortex.agentContextMenu', { role: 'tester' })).toBe('Агент: tester');
  });

  it('toggleLanguage switches between en and ru correctly', () => {
    setLanguage('en');
    useLanguageStore.getState().toggleLanguage();
    expect(currentLanguage()).toBe('ru');

    useLanguageStore.getState().toggleLanguage();
    expect(currentLanguage()).toBe('en');
  });

  it('translate helper supports explicit language override', () => {
    expect(translate('en', 'common.cancel')).toBe('Cancel');
    expect(translate('ru', 'common.cancel')).toBe('Отмена');
  });

  it('interpolate replaces all occurrences of parameters', () => {
    expect(interpolate('Hello {name}, your role is {role}', { name: 'Alice', role: 'Dev' }))
      .toBe('Hello Alice, your role is Dev');
  });
});