// Updated palette + materials for Family Room v2
// Midwestern-natural: muted creams, oat, hemp, sage, rust, walnut.
// Daylight mode: warm amber monochrome (inspired by Daylight Computer).

// Vintage-textile palette — Finnish quilt meets Mexican blanket.
// Unexpected warm colors that still feel soft. Comfort, not minimalism.
const PALETTE = {
  // base — a warm, dusty butter, not cream
  paper:    '#f4dfb2',   // butter
  cream:    '#eccf94',
  muslin:   '#e7bf82',
  hemp:     '#d6a362',
  // accents — textile hues
  clay:     '#e38c6e',   // clay pink
  rose:     '#d16b6b',   // washed rose
  claret:   '#8f3a3a',   // deep berry
  mustard:  '#d79a3e',   // mustard yellow
  moss:     '#7a8a4a',   // olive-moss
  pine:     '#3e5a4a',   // deep forest
  lavender: '#a48db0',   // dusty lavender
  terra:    '#b0492e',   // burnt terra rosa
  rust:     '#c26a3e',
  // wood
  walnut:   '#6d4224',
  walnutD:  '#3a2010',
  oak:      '#b98245',
  // type — warmer, more red-brown
  ink:      '#3a1810',
  inkSoft:  '#6a3a22',
  inkMute:  '#a07050',
  // daylight — a warmer, pinker amber (less greenish)
  dayBg:    '#eab675',
  dayInk:   '#3a1810',
  dayInkSoft: '#5a2818',
  dayAccent:'#7a3020',
};

// ─── Refined texture defs (no harsh noise — softer grains) ───
function MaterialDefs() {
  return (
    <svg width="0" height="0" style={{ position: 'absolute' }} aria-hidden>
      <defs>
        {/* soft linen — gentle, not speckly */}
        <filter id="linen2" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="turbulence" baseFrequency="0.03 0.5" numOctaves="2" seed="11" />
          <feColorMatrix values="0 0 0 0 0.52  0 0 0 0 0.42  0 0 0 0 0.28  0 0 0 0.1 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>
        {/* wool — slightly fuzzier */}
        <filter id="wool" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.7" numOctaves="2" seed="5" />
          <feColorMatrix values="0 0 0 0 0.3  0 0 0 0 0.22  0 0 0 0 0.14  0 0 0 0.12 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>
        {/* wood grain — long horizontal */}
        <filter id="wood2" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="turbulence" baseFrequency="0.008 0.4" numOctaves="3" seed="9" />
          <feColorMatrix values="0 0 0 0 0.4  0 0 0 0 0.26  0 0 0 0 0.16  0 0 0 0.5 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>
        {/* paper grain — very fine */}
        <filter id="paper" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="1.4" numOctaves="1" seed="2" />
          <feColorMatrix values="0 0 0 0 0.4  0 0 0 0 0.32  0 0 0 0 0.2  0 0 0 0.06 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>
        {/* daylight paper noise — amber monochrome */}
        <filter id="daypaper" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="1.2" numOctaves="1" seed="6" />
          <feColorMatrix values="0 0 0 0 0.3  0 0 0 0 0.2  0 0 0 0 0.1  0 0 0 0.09 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>

        {/* fluted glass — vertical bands */}
        <pattern id="flutes" patternUnits="userSpaceOnUse" width="14" height="100">
          <rect width="14" height="100" fill="rgba(255,255,255,0.0)" />
          <rect x="0" y="0" width="7" height="100" fill="rgba(255,255,255,0.08)" />
          <rect x="6" y="0" width="1" height="100" fill="rgba(0,0,0,0.05)" />
          <rect x="13" y="0" width="1" height="100" fill="rgba(255,255,255,0.18)" />
        </pattern>

        {/* subtle window light for daylight mode */}
        <linearGradient id="daywash" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#f7dfa8" stopOpacity="1" />
          <stop offset="100%" stopColor="#d9b67a" stopOpacity="1" />
        </linearGradient>
      </defs>
    </svg>
  );
}

// ─── Warmer, more textile-inspired wash ───
function DaylightWash({ intensity = 1, mode = 'natural' }) {
  if (mode === 'daylight') {
    return (
      <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none', overflow: 'hidden', zIndex: 1 }}>
        <div style={{ position: 'absolute', inset: 0, background: `linear-gradient(180deg, #f0c88a 0%, #eab675 60%, #d99858 100%)` }} />
        <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.7, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
          <rect width="100%" height="100%" filter="url(#daypaper)" fill="black" />
        </svg>
      </div>
    );
  }
  return (
    <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none', overflow: 'hidden', zIndex: 1 }}>
      {/* top right: warm butter-rose glow */}
      <div style={{
        position: 'absolute', top: '-15%', right: '-10%', width: '85%', height: '70%',
        background: `radial-gradient(ellipse at top right, rgba(255,220,160,0.9) 0%, rgba(240,160,130,0.25) 40%, rgba(200,100,90,0) 70%)`,
        opacity: 0.8 * intensity, mixBlendMode: 'screen',
      }} />
      {/* bottom-left: dusty claret shadow */}
      <div style={{
        position: 'absolute', bottom: '-25%', left: '-15%', width: '80%', height: '60%',
        background: `radial-gradient(ellipse at bottom left, rgba(140,60,60,0.22) 0%, transparent 60%)`,
        opacity: 0.55 * intensity, mixBlendMode: 'multiply',
      }} />
      {/* subtle lavender in the corner */}
      <div style={{
        position: 'absolute', top: '40%', left: '-5%', width: '40%', height: '30%',
        background: `radial-gradient(ellipse, rgba(164,141,176,0.25) 0%, transparent 70%)`,
        opacity: 0.4 * intensity, mixBlendMode: 'multiply',
      }} />
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.35, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#paper)" fill="black" />
      </svg>
    </div>
  );
}

// ─── Wood surface v2 — warmer, more natural ───
function Wood2({ tone = 'walnut', radius = 0, style = {}, children }) {
  const tones = {
    oak:    { base: '#d6b482', dark: '#a88050', shadow: '#6e5030' },
    walnut: { base: '#8a6240', dark: '#5a3d24', shadow: '#2e1c0e' },
    pine:   { base: '#e6ca9a', dark: '#c19d68', shadow: '#8a6538' },
    cherry: { base: '#b0704a', dark: '#7a4828', shadow: '#3e2010' },
  };
  const t = tones[tone] || tones.walnut;
  return (
    <div style={{
      position: 'relative', borderRadius: radius, overflow: 'hidden',
      background: `linear-gradient(178deg, ${t.base} 0%, ${t.dark} 75%, ${t.shadow} 100%)`,
      boxShadow: 'inset 0 1px 0 rgba(255,240,210,0.35)',
      ...style,
    }}>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.55, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#wood2)" fill="black" />
      </svg>
      <div style={{ position: 'relative', zIndex: 2 }}>{children}</div>
    </div>
  );
}

// ─── Fabric surface — richer textile tones ───
function Fabric({ tone = 'butter', radius = 0, style = {}, children }) {
  const tones = {
    butter:   '#f4dfb2',
    cream:    '#eccf94',
    muslin:   '#e7bf82',
    clay:     '#e38c6e',
    rose:     '#d89a96',
    claret:   '#8f3a3a',
    mustard:  '#d79a3e',
    moss:     '#8a9a5a',
    pine:     '#3e5a4a',
    lavender: '#b8a4c0',
    terra:    '#b0492e',
    // legacy aliases so older screens still render
    oat:      '#eccf94',
    sage:     '#8a9a5a',
    hemp:     '#d6a362',
    wool:     '#b8a4c0',
    rust:     '#c26a3e',
  };
  const bg = tones[tone] || tones.butter;
  return (
    <div style={{
      position: 'relative', borderRadius: radius, overflow: 'hidden', background: bg,
      ...style,
    }}>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.65, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#linen2)" fill="black" />
      </svg>
      <div style={{ position: 'relative', zIndex: 2 }}>{children}</div>
    </div>
  );
}

// ─── Fluted glass — warmer, pinker tints ───
function FlutedGlass({ tint = 'butter', radius = 16, style = {}, children }) {
  const tints = {
    butter:  'rgba(244, 223, 178, 0.55)',
    rose:    'rgba(216, 154, 150, 0.35)',
    clay:    'rgba(227, 140, 110, 0.3)',
    mustard: 'rgba(215, 154, 62, 0.28)',
    moss:    'rgba(138, 154, 90, 0.28)',
    clear:   'rgba(252, 238, 208, 0.4)',
    // aliases
    warm:    'rgba(244, 223, 178, 0.55)',
    cool:    'rgba(184, 164, 192, 0.35)',
    amber:   'rgba(227, 140, 110, 0.3)',
  };
  return (
    <div style={{
      position: 'relative', borderRadius: radius, overflow: 'hidden',
      background: tints[tint] || tints.butter,
      backdropFilter: 'blur(18px) saturate(170%)',
      WebkitBackdropFilter: 'blur(18px) saturate(170%)',
      boxShadow: 'inset 0 1px 0 rgba(255,240,210,0.55), inset 0 -1px 0 rgba(90,40,30,0.06), 0 0 0 0.5px rgba(140,70,50,0.15)',
      ...style,
    }}>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.7, mixBlendMode: 'overlay', pointerEvents: 'none' }}>
        <rect width="100%" height="100%" fill="url(#flutes)" />
      </svg>
      <div style={{ position: 'relative', zIndex: 2 }}>{children}</div>
    </div>
  );
}

// ─── Natural photo placeholder — muted earth tones ───
function NaturalPhoto({ label, hue = 30, sat = 22, light = 62, style = {}, ratio = 1, mode = 'natural' }) {
  if (mode === 'daylight') {
    // amber monochrome — use hue as brightness variance
    const base = 55 + (hue % 30);
    return (
      <div style={{
        position: 'relative', width: '100%', aspectRatio: ratio, overflow: 'hidden',
        background: `linear-gradient(160deg, hsl(35, 50%, ${base + 8}%), hsl(32, 55%, ${base}%), hsl(30, 60%, ${base - 15}%))`,
        ...style,
      }}>
        <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.5, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
          <rect width="100%" height="100%" filter="url(#daypaper)" fill="black" />
        </svg>
        {label && (
          <div style={{
            position: 'absolute', bottom: 8, left: 10,
            fontFamily: 'ui-monospace, monospace', fontSize: 10,
            color: 'rgba(60,40,20,0.7)', letterSpacing: 0.5,
          }}>{label}</div>
        )}
      </div>
    );
  }
  const bg = `hsl(${hue}, ${sat}%, ${light}%)`;
  const fg = `hsl(${hue}, ${sat + 6}%, ${light - 14}%)`;
  const dark = `hsl(${hue}, ${sat + 10}%, ${light - 30}%)`;
  return (
    <div style={{
      position: 'relative', width: '100%', aspectRatio: ratio, overflow: 'hidden',
      background: `linear-gradient(160deg, ${bg} 0%, ${fg} 55%, ${dark} 100%)`,
      ...style,
    }}>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.12, mixBlendMode: 'overlay' }} preserveAspectRatio="none">
        <defs>
          <pattern id={`ns-${hue}-${sat}`} patternUnits="userSpaceOnUse" width="10" height="10" patternTransform="rotate(28)">
            <rect width="5" height="10" fill="#fff" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill={`url(#ns-${hue}-${sat})`} />
      </svg>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.3, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#paper)" fill="black" />
      </svg>
      {label && (
        <div style={{
          position: 'absolute', bottom: 8, left: 10,
          fontFamily: 'ui-monospace, monospace', fontSize: 10,
          color: 'rgba(255,245,220,0.85)', letterSpacing: 0.5,
          textShadow: '0 1px 2px rgba(40,20,10,0.4)',
        }}>{label}</div>
      )}
    </div>
  );
}

Object.assign(window, { PALETTE, MaterialDefs, DaylightWash, Wood2, Fabric, FlutedGlass, NaturalPhoto });
