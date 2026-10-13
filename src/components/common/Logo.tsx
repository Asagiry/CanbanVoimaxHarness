import { useId } from 'react';

interface LogoProps {
  /** Size in pixels (width and height), default 24 */
  size?: number;
  /** Optional custom CSS classes */
  className?: string;
  /** Whether to animate the quantum spark and pulse */
  animated?: boolean;
  /** Show enhanced background glow aura */
  glow?: boolean;
}

export default function Logo({
  size = 24,
  className = '',
  animated = false,
  glow = false,
}: LogoProps) {
  const uid = useId().replace(/:/g, '_');

  const colLeftId = `colLeft_${uid}`;
  const colCenterId = `colCenter_${uid}`;
  const colRightId = `colRight_${uid}`;
  const pulseId = `pulse_${uid}`;
  const glowId = `glow_${uid}`;
  const pillarGlowId = `pillarGlow_${uid}`;

  return (
    <div
      className={`inline-flex items-center justify-center shrink-0 relative select-none ${className}`}
      style={{ width: size, height: size }}
    >
      {glow && (
        <div
          className="absolute inset-0 rounded-full blur-md opacity-40 pointer-events-none"
          style={{
            background:
              'radial-gradient(circle, rgba(6,182,212,0.6) 0%, rgba(139,92,246,0.3) 50%, transparent 75%)',
          }}
        />
      )}
      <svg
        viewBox="0 0 48 48"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        className="w-full h-full relative z-10 overflow-visible"
      >
        <defs>
          {/* Column 1: Violet -> Electric Cyan */}
          <linearGradient id={colLeftId} x1="7" y1="14" x2="16" y2="38" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#8b5cf6" />
            <stop offset="60%" stopColor="#6366f1" />
            <stop offset="100%" stopColor="#06b6d4" />
          </linearGradient>

          {/* Column 2: Cyber Cyan -> Neon Emerald -> Mint */}
          <linearGradient id={colCenterId} x1="19.75" y1="8" x2="28.25" y2="40" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#06b6d4" />
            <stop offset="45%" stopColor="#10b981" />
            <stop offset="100%" stopColor="#34d399" />
          </linearGradient>

          {/* Column 3: Emerald -> Warm Amber -> Coral */}
          <linearGradient id={colRightId} x1="32.5" y1="16" x2="41" y2="38" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#10b981" />
            <stop offset="55%" stopColor="#f59e0b" />
            <stop offset="100%" stopColor="#fb923c" />
          </linearGradient>

          {/* Quantum Pulse Gradient */}
          <linearGradient id={pulseId} x1="4" y1="24" x2="44" y2="24" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#8b5cf6" stopOpacity="0.2" />
            <stop offset="25%" stopColor="#06b6d4" stopOpacity="0.9" />
            <stop offset="50%" stopColor="#ffffff" stopOpacity="1" />
            <stop offset="75%" stopColor="#10b981" stopOpacity="0.9" />
            <stop offset="100%" stopColor="#f59e0b" stopOpacity="0.2" />
          </linearGradient>

          {/* Glow Filters */}
          <filter id={glowId} x="-60%" y="-60%" width="220%" height="220%">
            <feGaussianBlur stdDeviation="2" result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>

          <filter id={pillarGlowId} x="-30%" y="-30%" width="160%" height="160%">
            <feGaussianBlur stdDeviation="1.5" result="blur" />
            <feColorMatrix type="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 0.35 0" />
            <feMerge>
              <feMergeNode />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {/* Ambient Glow Behind Central Core */}
        <circle cx="24" cy="23" r="14" fill="#06b6d4" opacity="0.12" filter={`url(#${glowId})`} />

        {/* === COLUMN 1: INGEST / BACKLOG (Violet -> Cyan) === */}
        <g opacity="0.92">
          {/* Top card */}
          <rect x="7" y="14" width="8.5" height="7.5" rx="2.5" fill={`url(#${colLeftId})`} />
          {/* Bottom card */}
          <rect x="7" y="23.5" width="8.5" height="14.5" rx="2.5" fill={`url(#${colLeftId})`} />
          {/* Subtle tactile highlight borders */}
          <rect x="7" y="14" width="8.5" height="7.5" rx="2.5" stroke="#ffffff" strokeWidth="0.75" strokeOpacity="0.35" />
          <rect x="7" y="23.5" width="8.5" height="14.5" rx="2.5" stroke="#ffffff" strokeWidth="0.75" strokeOpacity="0.2" />
        </g>

        {/* === COLUMN 2: ACTIVE REASONING / CORE (Cyan -> Emerald -> Mint) === */}
        <g filter={`url(#${pillarGlowId})`}>
          {/* Top card */}
          <rect x="19.75" y="8" width="8.5" height="11" rx="2.5" fill={`url(#${colCenterId})`} />
          {/* Bottom card */}
          <rect x="19.75" y="21" width="8.5" height="19" rx="2.5" fill={`url(#${colCenterId})`} />
          {/* Subtle tactile highlight borders */}
          <rect x="19.75" y="8" width="8.5" height="11" rx="2.5" stroke="#ffffff" strokeWidth="0.75" strokeOpacity="0.45" />
          <rect x="19.75" y="21" width="8.5" height="19" rx="2.5" stroke="#ffffff" strokeWidth="0.75" strokeOpacity="0.3" />
        </g>

        {/* === COLUMN 3: DEPLOYED / PRODUCTION (Emerald -> Amber) === */}
        <g opacity="0.92">
          {/* Top card */}
          <rect x="32.5" y="16" width="8.5" height="13.5" rx="2.5" fill={`url(#${colRightId})`} />
          {/* Bottom card */}
          <rect x="32.5" y="31.5" width="8.5" height="6.5" rx="2.5" fill={`url(#${colRightId})`} />
          {/* Subtle tactile highlight borders */}
          <rect x="32.5" y="16" width="8.5" height="13.5" rx="2.5" stroke="#ffffff" strokeWidth="0.75" strokeOpacity="0.35" />
          <rect x="32.5" y="31.5" width="8.5" height="6.5" rx="2.5" stroke="#ffffff" strokeWidth="0.75" strokeOpacity="0.2" />
        </g>

        {/* === QUANTUM WAVEGUIDE / IMPULSE === */}
        <path
          d="M 4 27.5 C 13 27.5 17 22.5 24 22.5 C 31 22.5 35 26.5 44 26.5"
          stroke={`url(#${pulseId})`}
          strokeWidth="2.5"
          strokeLinecap="round"
          className={animated ? 'animate-pulse' : ''}
        />
        <path
          d="M 4 27.5 C 13 27.5 17 22.5 24 22.5 C 31 22.5 35 26.5 44 26.5"
          stroke="#ffffff"
          strokeWidth="1"
          strokeLinecap="round"
          strokeOpacity="0.9"
        />

        {/* === CENTRAL QUANTUM SPARK OF MIND === */}
        <g className={animated ? 'transition-transform duration-700 hover:scale-110' : ''}>
          {/* 4-pointed radiant flare */}
          <path
            d="M 24 16.5 C 24 20.2 26.2 22.5 29.5 22.5 C 26.2 22.5 24 24.8 24 28.5 C 24 24.8 21.8 22.5 18.5 22.5 C 21.8 22.5 24 20.2 24 16.5 Z"
            fill="#ffffff"
            filter={`url(#${glowId})`}
          />
          {/* High intensity core diamond */}
          <polygon points="24,19 26.5,22.5 24,26 21.5,22.5" fill="#ffffff" />
          {/* Pure core hot-point */}
          <circle cx="24" cy="22.5" r="1.2" fill="#ffffff" />
        </g>

        {/* Nano-satellites (active agent signals) */}
        <circle cx="11.25" cy="11.5" r="1" fill="#c084fc" opacity="0.8" />
        <circle cx="36.75" cy="13.5" r="1" fill="#fbbf24" opacity="0.8" />
      </svg>
    </div>
  );
}
