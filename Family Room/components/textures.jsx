// Reusable SVG-based textures and filters. Keep SVGs simple (repeating noise/patterns),
// no hand-drawn illustration.

function TextureDefs() {
  return (
    <svg width="0" height="0" style={{ position: 'absolute' }} aria-hidden>
      <defs>
        {/* grain — fine paper/fabric noise */}
        <filter id="grain" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="3" />
          <feColorMatrix values="0 0 0 0 0.25  0 0 0 0 0.18  0 0 0 0 0.12  0 0 0 0.35 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>
        {/* linen — woven fabric */}
        <filter id="linen" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="turbulence" baseFrequency="0.04 0.7" numOctaves="2" seed="7" />
          <feColorMatrix values="0 0 0 0 0.92  0 0 0 0 0.86  0 0 0 0 0.74  0 0 0 0.18 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>
        {/* wood grain — long horizontal bands */}
        <filter id="wood" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="turbulence" baseFrequency="0.012 0.45" numOctaves="3" seed="4" />
          <feColorMatrix values="0 0 0 0 0.42  0 0 0 0 0.28  0 0 0 0 0.18  0 0 0 0.55 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>
        {/* sunlight pool — soft radial gradient */}
        <radialGradient id="sunpool" cx="70%" cy="10%" r="90%">
          <stop offset="0%" stopColor="#fff4d6" stopOpacity="0.85" />
          <stop offset="40%" stopColor="#ffe3ae" stopOpacity="0.35" />
          <stop offset="100%" stopColor="#ffd89a" stopOpacity="0" />
        </radialGradient>
        {/* sun rays — barely visible angled stripes */}
        <pattern id="rays" patternUnits="userSpaceOnUse" width="280" height="600" patternTransform="rotate(-22)">
          <rect width="280" height="600" fill="transparent" />
          <rect x="0" y="0" width="60" height="600" fill="#fff2d0" opacity="0.08" />
          <rect x="110" y="0" width="30" height="600" fill="#fff6e0" opacity="0.05" />
          <rect x="200" y="0" width="80" height="600" fill="#fff2d0" opacity="0.06" />
        </pattern>
      </defs>
    </svg>
  );
}

// Utility: a full-bleed sunlight wash — put inside any relative parent
function SunlightWash({ intensity = 1, warmth = 'afternoon' }) {
  const tones = {
    morning:   { a: '#fff8e8', b: '#ffe9c2' },
    midday:    { a: '#fffbf0', b: '#fff0d0' },
    afternoon: { a: '#fff1d0', b: '#ffcf95' },
    golden:    { a: '#ffe0a8', b: '#ffb878' },
    evening:   { a: '#3a2a20', b: '#120a08' },
  };
  const t = tones[warmth] || tones.afternoon;
  return (
    <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none', overflow: 'hidden', zIndex: 1 }}>
      <div style={{
        position: 'absolute', top: '-20%', right: '-15%', width: '90%', height: '80%',
        background: `radial-gradient(ellipse at top right, ${t.a} 0%, ${t.b}00 55%)`,
        opacity: 0.9 * intensity, mixBlendMode: 'screen',
      }} />
      <div style={{
        position: 'absolute', top: 0, left: 0, right: 0, bottom: 0,
        background: `linear-gradient(200deg, ${t.a}${warmth === 'evening' ? '00' : '88'} 0%, transparent 45%)`,
        opacity: 0.55 * intensity,
      }} />
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.5 * intensity, mixBlendMode: 'soft-light' }}>
        <rect width="100%" height="100%" fill="url(#rays)" />
      </svg>
    </div>
  );
}

// A wood panel surface — configurable tone
function WoodSurface({ tone = 'walnut', radius = 0, style = {}, children }) {
  const tones = {
    oak:     { base: '#d9b784', dark: '#b08850', shadow: '#8a6536' },
    walnut:  { base: '#8d6240', dark: '#5d3d24', shadow: '#3a2414' },
    pine:    { base: '#e8c999', dark: '#c19a66', shadow: '#99754b' },
    ebony:   { base: '#3a2a20', dark: '#1e140e', shadow: '#0c0705' },
  };
  const t = tones[tone] || tones.walnut;
  return (
    <div style={{
      position: 'relative', borderRadius: radius, overflow: 'hidden',
      background: `linear-gradient(175deg, ${t.base} 0%, ${t.dark} 70%, ${t.shadow} 100%)`,
      ...style,
    }}>
      {/* wood grain lines */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.55, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#wood)" fill="black" />
      </svg>
      {/* long knots / streaks */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.3 }} preserveAspectRatio="none">
        <defs>
          <linearGradient id={`streak-${tone}`} x1="0" y1="0" x2="1" y2="0.1">
            <stop offset="0%" stopColor={t.shadow} stopOpacity="0" />
            <stop offset="50%" stopColor={t.shadow} stopOpacity="0.4" />
            <stop offset="100%" stopColor={t.shadow} stopOpacity="0" />
          </linearGradient>
        </defs>
        <rect x="0" y="18%" width="100%" height="1.5" fill={`url(#streak-${tone})`} />
        <rect x="0" y="43%" width="100%" height="2" fill={`url(#streak-${tone})`} />
        <rect x="0" y="72%" width="100%" height="1.2" fill={`url(#streak-${tone})`} />
      </svg>
      {/* top highlight */}
      <div style={{
        position: 'absolute', top: 0, left: 0, right: 0, height: '40%',
        background: `linear-gradient(180deg, rgba(255,245,220,0.25) 0%, transparent 100%)`,
        pointerEvents: 'none',
      }} />
      <div style={{ position: 'relative', zIndex: 2 }}>{children}</div>
    </div>
  );
}

// A linen/fabric surface
function LinenSurface({ tone = 'cream', radius = 0, style = {}, children }) {
  const tones = {
    cream:  '#f5ead3',
    oat:    '#e8d9ba',
    sage:   '#c9d1b8',
    rose:   '#e6c8b8',
    stone:  '#d8cfc0',
  };
  const bg = tones[tone] || tones.cream;
  return (
    <div style={{
      position: 'relative', borderRadius: radius, overflow: 'hidden', background: bg,
      ...style,
    }}>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.8, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#linen)" fill="black" />
      </svg>
      {/* warp threads — vertical */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.12 }} preserveAspectRatio="none">
        <pattern id={`warp-${tone}`} patternUnits="userSpaceOnUse" width="3" height="3">
          <rect width="1" height="3" fill="#7a6248" />
        </pattern>
        <rect width="100%" height="100%" fill={`url(#warp-${tone})`} />
      </svg>
      <div style={{ position: 'relative', zIndex: 2 }}>{children}</div>
    </div>
  );
}

// A placeholder photo — soft gradient stripes with monospace caption
function PhotoPlaceholder({ label, hue = 30, style = {}, ratio = 1 }) {
  const bg = `hsl(${hue}, 35%, 72%)`;
  const fg = `hsl(${hue}, 40%, 55%)`;
  const dark = `hsl(${hue}, 45%, 40%)`;
  return (
    <div style={{
      position: 'relative', width: '100%', aspectRatio: ratio, overflow: 'hidden',
      background: `linear-gradient(160deg, ${bg} 0%, ${fg} 50%, ${dark} 100%)`,
      ...style,
    }}>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.18, mixBlendMode: 'overlay' }} preserveAspectRatio="none">
        <defs>
          <pattern id={`stripes-${hue}`} patternUnits="userSpaceOnUse" width="8" height="8" patternTransform="rotate(35)">
            <rect width="4" height="8" fill="#fff" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill={`url(#stripes-${hue})`} />
      </svg>
      {label && (
        <div style={{
          position: 'absolute', bottom: 8, left: 10,
          fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
          fontSize: 10, color: 'rgba(255,255,255,0.85)', letterSpacing: 0.5,
          textShadow: '0 1px 2px rgba(0,0,0,0.3)',
        }}>{label}</div>
      )}
    </div>
  );
}

Object.assign(window, { TextureDefs, SunlightWash, WoodSurface, LinenSurface, PhotoPlaceholder });
