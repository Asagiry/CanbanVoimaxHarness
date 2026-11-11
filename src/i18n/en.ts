import { enCore } from './en/core';
import { enDrawer } from './en/drawer';
import { enSettingsTools } from './en/settingsTools';
import { enCortex } from './en/cortex';

export const en = {
  ...enCore,
  ...enDrawer,
  ...enSettingsTools,
  ...enCortex,
} as const;
