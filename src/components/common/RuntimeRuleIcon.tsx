export interface RuntimeRuleIconProps {
  className?: string;
}

export function isRuntimeRule(fileName?: string | null): boolean {
  const clean = (fileName ?? '').trim().toLowerCase();
  return clean === 'runtime.md' || clean === 'runtime';
}

export function RuntimeRuleIcon({ className = 'h-3.5 w-3.5 shrink-0 block' }: RuntimeRuleIconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <rect
        x="4"
        y="4"
        width="16"
        height="16"
        rx="2.5"
        fill="currentColor"
        fillOpacity="0.12"
        stroke="currentColor"
        strokeWidth="1.75"
      />
      {/* Top pins */}
      <line x1="9" y1="1.5" x2="9" y2="4" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      <line x1="15" y1="1.5" x2="15" y2="4" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      {/* Bottom pins */}
      <line x1="9" y1="20" x2="9" y2="22.5" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      <line x1="15" y1="20" x2="15" y2="22.5" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      {/* Left pins */}
      <line x1="1.5" y1="9" x2="4" y2="9" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      <line x1="1.5" y1="15" x2="4" y2="15" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      {/* Right pins */}
      <line x1="20" y1="9" x2="22.5" y2="9" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      <line x1="20" y1="15" x2="22.5" y2="15" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      {/* Embedded prompt glyph */}
      <path
        d="M8.5 9.5L11.5 12L8.5 14.5"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <line
        x1="13.5"
        y1="14.5"
        x2="16"
        y2="14.5"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

export default RuntimeRuleIcon;
