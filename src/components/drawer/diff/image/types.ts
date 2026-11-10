export type ImageViewMode = '2up' | 'split' | 'onion' | 'new' | 'old';

export interface ImageDimensions {
  width: number;
  height: number;
}

export interface LightboxState {
  url: string;
  title: string;
}

export const CHECKERBOARD_CLASSES =
  'relative flex items-center justify-center p-2 rounded overflow-hidden select-none ' +
  'bg-[#f8fafc] dark:bg-[#090d16] ' +
  '[background-image:linear-gradient(45deg,#e2e8f0_25%,transparent_25%),linear-gradient(-45deg,#e2e8f0_25%,transparent_25%),linear-gradient(45deg,transparent_75%,#e2e8f0_75%),linear-gradient(-45deg,transparent_75%,#e2e8f0_75%)] ' +
  'dark:[background-image:linear-gradient(45deg,#1c2128_25%,transparent_25%),linear-gradient(-45deg,#1c2128_25%,transparent_25%),linear-gradient(45deg,transparent_75%,#1c2128_75%),linear-gradient(-45deg,transparent_75%,#1c2128_75%)] ' +
  '[background-size:16px_16px] [background-position:0_0,0_8px,8px_-8px,-8px_0px] border border-[#e5e7eb] dark:border-[#212126]';
