const SAFE_LINK_SCHEMES = new Set(['http:', 'https:', 'mailto:', 'tel:']);
const SAFE_IMAGE_SCHEMES = new Set(['http:', 'https:', 'asset:', 'tauri:', 'blob:']);
const SAFE_RASTER_DATA_IMAGE_RE = /^data:image\/(?:png|jpe?g|gif|webp|bmp|ico);base64,[a-z0-9+/=]+$/i;
const SCHEME_RE = /^([a-z][a-z0-9+.-]*:)/i;

export interface SanitizeMarkdownUrlOptions {
  allowDataImage?: boolean;
}

/**
 * Strips ASCII control characters and whitespace so obfuscated schemes
 * such as `java\tscript:` or `\u0001vbscript:` cannot bypass protocol checks.
 */
function normalizeUrlForSchemeCheck(raw: string): string {
  let out = '';
  for (let i = 0; i < raw.length; i += 1) {
    const code = raw.charCodeAt(i);
    if (code > 0x20 && code !== 0x7f) {
      out += raw[i];
    }
  }
  return out;
}

/**
 * Validates whether a Markdown link or image URL uses a safe scheme (or relative path).
 * Blocks `javascript:`, `vbscript:`, `data:text/html`, `data:image/svg+xml`, etc.
 */
export function isSafeMarkdownUrl(
  rawUrl: string,
  options: SanitizeMarkdownUrlOptions = {}
): boolean {
  if (typeof rawUrl !== 'string') return false;
  const trimmed = rawUrl.trim();
  if (!trimmed) return false;

  const normalized = normalizeUrlForSchemeCheck(trimmed);
  if (!normalized) return false;

  // Protocol-relative URLs (//evil.com) or scheme-less relative paths
  if (normalized.startsWith('//')) {
    return true;
  }

  const beforeQueryOrHash = normalized.split(/[/?#]/, 1)[0];
  const schemeMatch = beforeQueryOrHash.match(SCHEME_RE);
  if (!schemeMatch) {
    return true;
  }

  const scheme = schemeMatch[1].toLowerCase();
  if (options.allowDataImage) {
    if (SAFE_IMAGE_SCHEMES.has(scheme)) return true;
    if (scheme === 'data:') {
      return SAFE_RASTER_DATA_IMAGE_RE.test(normalized);
    }
    return false;
  }

  return SAFE_LINK_SCHEMES.has(scheme);
}

/**
 * Returns the original trimmed URL if safe, or `'#'` (for links) / `''` (for images).
 */
export function sanitizeMarkdownUrl(
  rawUrl: string,
  options: SanitizeMarkdownUrlOptions = {}
): string {
  if (!isSafeMarkdownUrl(rawUrl, options)) {
    return options.allowDataImage ? '' : '#';
  }
  return rawUrl.trim();
}
