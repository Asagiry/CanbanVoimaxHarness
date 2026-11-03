export function formatDurationCompact(seconds?: number | null): string {
  if (seconds == null || seconds <= 0 || !Number.isFinite(seconds)) {
    return '0s';
  }
  const total = Math.round(seconds);
  if (total < 60) {
    return `${total}s`;
  }
  if (total < 3600) {
    const mins = Math.floor(total / 60);
    const secs = total % 60;
    return `${mins}m${secs}s`;
  }
  const hours = Math.floor(total / 3600);
  const remainingMins = Math.floor((total % 3600) / 60);
  return `${hours}h${remainingMins}m`;
}

export function formatTtft(ms?: number | null): string {
  if (ms == null || ms <= 0 || !Number.isFinite(ms)) {
    return '—';
  }
  return `${(ms / 1000).toFixed(1)}s`;
}

export function formatTps(tps?: number | null): string {
  if (tps == null || tps <= 0 || !Number.isFinite(tps)) {
    return '—';
  }
  const rounded = tps >= 10 ? Math.round(tps) : tps.toFixed(1);
  return `${rounded} tok/s`;
}

export function formatTokensWithUnit(tokens?: number | null): string {
  if (tokens == null || tokens <= 0 || !Number.isFinite(tokens)) {
    return '0 tok';
  }
  return `${Math.round(tokens).toLocaleString('en-US')} tok`;
}

export function formatCompactTokens(tokens?: number | null): string {
  if (tokens == null || tokens <= 0 || !Number.isFinite(tokens)) {
    return '0 tok';
  }
  const roundVal = Math.round(tokens);
  if (roundVal >= 1_000_000) {
    const millions = (roundVal / 1_000_000).toFixed(1).replace('.0', '');
    return `${millions}M tok`;
  }
  if (roundVal >= 1_000) {
    const thousands = (roundVal / 1_000).toFixed(1).replace('.0', '');
    return `${thousands}k tok`;
  }
  return `${roundVal} tok`;
}

export function pluralize(
  n: number,
  one: string,
  few: string,
  many: string,
  lang = 'ru'
): string {
  const abs = Math.abs(Math.round(n));
  if (lang === 'en') {
    return abs === 1 ? one : few;
  }
  const mod100 = abs % 100;
  const mod10 = abs % 10;
  if (mod100 >= 11 && mod100 <= 19) return many;
  if (mod10 === 1) return one;
  if (mod10 >= 2 && mod10 <= 4) return few;
  return many;
}
