import React from 'react';

export interface LogoProps extends React.SVGProps<SVGSVGElement> {
  size?: number | string;
  variant?: 'badge' | 'glyph';
  className?: string;
}

export function LogoGlyph({ className = '', ...props }: React.SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      {...props}
    >
      {/* Terminal Delta Glyph (▲) */}
      <polygon points="16,5 23,15 9,15" fill="currentColor" />

      {/* Geometric Kanban Columns */}
      <rect x="6.5" y="18" width="5" height="9" rx="1.5" fill="currentColor" fillOpacity="0.35" />
      <rect x="13.5" y="18" width="5" height="9" rx="1.5" fill="currentColor" />
      <rect x="20.5" y="18" width="5" height="9" rx="1.5" fill="currentColor" fillOpacity="0.7" />
    </svg>
  );
}

export default function Logo({
  size = 20,
  variant = 'badge',
  className = '',
  ...props
}: LogoProps) {
  if (variant === 'glyph') {
    return <LogoGlyph width={size} height={size} className={className} {...props} />;
  }

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={`shrink-0 ${className}`}
      {...props}
    >
      {/* Obsidian Badge Surface with 6px Geist Radius */}
      <rect width="32" height="32" rx="6" fill="#171717" />
      <rect x="0.5" y="0.5" width="31" height="31" rx="5.5" stroke="#27272a" strokeWidth="1" />

      {/* Terminal Delta Glyph (▲) */}
      <polygon points="16,6.5 21.5,14.5 10.5,14.5" fill="#ffffff" />

      {/* Geometric Kanban Columns */}
      <rect x="7" y="17.5" width="4.5" height="8" rx="1.2" fill="#ffffff" fillOpacity="0.38" />
      <rect x="13.75" y="17.5" width="4.5" height="8" rx="1.2" fill="#ffffff" />
      <rect x="20.5" y="17.5" width="4.5" height="8" rx="1.2" fill="#ffffff" fillOpacity="0.72" />
    </svg>
  );
}

export function LogoWithWordmark({
  size = 20,
  className = '',
}: {
  size?: number | string;
  className?: string;
}) {
  return (
    <div className={`inline-flex items-center gap-2 select-none ${className}`}>
      <Logo size={size} />
      <div className="flex items-baseline gap-1.5">
        <span className="font-semibold text-xs tracking-tight text-[#171717] dark:text-[#f0f6fc] font-sans">
          CanbanVoimax
        </span>
        <span className="font-mono text-[9px] uppercase tracking-wider text-[#8f8f8f] dark:text-[#8b949e] px-1 py-0.2 rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22]">
          HARNESS
        </span>
      </div>
    </div>
  );
}
