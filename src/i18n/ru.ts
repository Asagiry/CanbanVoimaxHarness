import type { TranslationDict } from './types';
import { ruCore } from './ru/core';
import { ruDrawer } from './ru/drawer';
import { ruSettingsTools } from './ru/settingsTools';
import { ruCortex } from './ru/cortex';

export const ru = {
  ...ruCore,
  ...ruDrawer,
  ...ruSettingsTools,
  ...ruCortex,
} as const satisfies TranslationDict;
