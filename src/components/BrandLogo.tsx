interface Props {
  className?: string;
}

export function BrandLogo({ className }: Props) {
  return (
    <svg
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <defs>
        <linearGradient id="apx-glass" x1="5" y1="3" x2="27" y2="29">
          <stop offset="0%" stopColor="#FFFFFF" />
          <stop offset="52%" stopColor="#F6F2EA" />
          <stop offset="100%" stopColor="#E7E0D4" />
        </linearGradient>
        <radialGradient id="apx-glow" cx="0" cy="0" r="1" gradientTransform="matrix(17 18 -18 17 11 8)">
          <stop offset="0%" stopColor="#FFFFFF" stopOpacity="0.95" />
          <stop offset="100%" stopColor="#FFFFFF" stopOpacity="0" />
        </radialGradient>
        <filter id="apx-shadow" x="-20%" y="-20%" width="140%" height="140%">
          <feDropShadow dx="0" dy="1.2" stdDeviation="1.4" floodColor="#2C241A" floodOpacity="0.18" />
        </filter>
      </defs>
      <rect x="2.25" y="2.25" width="27.5" height="27.5" rx="7.5" fill="url(#apx-glass)" filter="url(#apx-shadow)" />
      <rect x="2.85" y="2.85" width="26.3" height="26.3" rx="7" fill="url(#apx-glow)" />
      <rect x="2.75" y="2.75" width="26.5" height="26.5" rx="7.15" stroke="#FFFFFF" strokeOpacity="0.72" strokeWidth="1" />
      <rect x="3.25" y="3.25" width="25.5" height="25.5" rx="6.65" stroke="#9B8D78" strokeOpacity="0.28" strokeWidth="0.7" />
      <path
        d="M8 16H12.25"
        stroke="#3D3932"
        strokeWidth="2.2"
        strokeLinecap="round"
      />
      <path
        d="M19.75 16H24"
        stroke="#3D3932"
        strokeWidth="2.2"
        strokeLinecap="round"
        opacity="0.72"
      />
      <path
        d="M18.75 13.75L22.4 10.1"
        stroke="#3D3932"
        strokeWidth="2.05"
        strokeLinecap="round"
        opacity="0.72"
      />
      <path
        d="M18.75 18.25L22.4 21.9"
        stroke="#3D3932"
        strokeWidth="2.05"
        strokeLinecap="round"
        opacity="0.72"
      />
      <path
        d="M12.1 12.9C12.1 11.9 12.9 11.1 13.9 11.1H18.1C19.1 11.1 19.9 11.9 19.9 12.9V19.1C19.9 20.1 19.1 20.9 18.1 20.9H13.9C12.9 20.9 12.1 20.1 12.1 19.1V12.9Z"
        fill="#3D3932"
      />
      <path
        d="M14.25 13.9H17.75M14.25 16H17.75M14.25 18.1H16.4"
        stroke="#F8F3EA"
        strokeWidth="1.1"
        strokeLinecap="round"
        opacity="0.9"
      />
      <circle cx="7.5" cy="16" r="1.65" fill="#F59E0B" stroke="#FFF7E6" strokeWidth="0.75" />
      <circle cx="23.75" cy="9.75" r="1.65" fill="#10B981" stroke="#ECFFF8" strokeWidth="0.75" />
      <circle cx="24.5" cy="16" r="1.65" fill="#6E6252" stroke="#F9F5ED" strokeWidth="0.75" />
      <circle cx="23.75" cy="22.25" r="1.65" fill="#F59E0B" stroke="#FFF7E6" strokeWidth="0.75" />
    </svg>
  );
}
