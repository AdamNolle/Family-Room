// Atmospheric layer — window light shadows, film grain, springy fabric components
// Aligned with the repo's Cozy Glass system (SF Rounded, ultraThinMaterial, spring 0.6/0.8).

// ─── SVG defs: film grain, window shadow, linen weave, fluted glass ───
function AtmosphereDefs() {
  return (
    <svg width="0" height="0" style={{ position: 'absolute' }} aria-hidden>
      <defs>
        {/* fine film grain — animated via filter seed on a fixed element is expensive;
            keep static but organic */}
        <filter id="grain" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="3" stitchTiles="stitch" />
          <feColorMatrix values="0 0 0 0 0.45  0 0 0 0 0.32  0 0 0 0 0.22  0 0 0 0.22 0" />
          <feComposite in2="SourceGraphic" operator="in" />
        </filter>
        <filter id="grainCoarse" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.55" numOctaves="1" seed="7" stitchTiles="stitch" />
          <feColorMatrix values="0 0 0 0 0.5  0 0 0 0 0.35  0 0 0 0 0.22  0 0 0 0.12 0" />
        </filter>
        {/* linen weave — crossed directional turbulence */}
        <filter id="weave" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="turbulence" baseFrequency="0.02 0.7" numOctaves="2" seed="4" />
          <feColorMatrix values="0 0 0 0 0.5  0 0 0 0 0.35  0 0 0 0 0.22  0 0 0 0.14 0" />
        </filter>
        <filter id="weaveH" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="turbulence" baseFrequency="0.7 0.02" numOctaves="2" seed="9" />
          <feColorMatrix values="0 0 0 0 0.5  0 0 0 0 0.35  0 0 0 0 0.22  0 0 0 0.12 0" />
        </filter>
        {/* fluted vertical bands */}
        <pattern id="flute" patternUnits="userSpaceOnUse" width="18" height="100">
          <rect width="18" height="100" fill="transparent" />
          <rect x="0" y="0" width="9" height="100" fill="rgba(255,240,215,0.14)" />
          <rect x="8" y="0" width="1" height="100" fill="rgba(90,40,20,0.06)" />
          <rect x="17" y="0" width="1" height="100" fill="rgba(255,245,220,0.22)" />
        </pattern>
      </defs>
    </svg>
  );
}

// ─── SUNLIGHT — golden hour + prism rainbow refraction (dreamcore / 90s photo) ───
function WindowLight({ intensity = 1, mode = 'natural' }) {
  // 'intensity' is still respected but this version leans into saturated golden hour.
  // Off = dawn cool cast; full = blown-out golden sun with lens flare + rainbow spill on floor.
  const I = Math.max(0.25, intensity); // never fully off — always SOME atmosphere
  return (
    <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none', overflow: 'hidden', zIndex: 1 }}>
      {/* GOLDEN SUN — warm orange hotspot through trees, upper-center-left */}
      <div style={{
        position: 'absolute', top: '-15%', left: '-5%', width: '70%', height: '70%',
        background: 'radial-gradient(circle 35% 35% at 35% 35%, rgba(255,240,180,1) 0%, rgba(255,200,110,0.9) 12%, rgba(255,150,80,0.55) 28%, rgba(240,100,70,0.3) 45%, rgba(180,70,120,0) 65%)',
        mixBlendMode: 'screen',
        opacity: I,
        filter: 'blur(2px)',
      }} />

      {/* LENS FLARE — anamorphic horizontal streak through the sun */}
      <div style={{
        position: 'absolute', top: '12%', left: '-30%', width: '150%', height: '4%',
        background: 'linear-gradient(90deg, transparent 0%, rgba(255,210,140,0.3) 25%, rgba(255,240,180,0.85) 42%, rgba(255,240,180,0.85) 48%, rgba(255,210,140,0.4) 70%, transparent 100%)',
        mixBlendMode: 'screen',
        filter: 'blur(3px)',
        opacity: 0.9 * I,
      }} />

      {/* PRISM RAINBOW STREAK — chromatic aberration / stained-glass floor spill */}
      {/* Rainbow beam angling down across the lower portion of the screen */}
      <div style={{
        position: 'absolute', top: '30%', left: '-20%', width: '140%', height: '60%',
        transform: 'rotate(8deg)',
        mixBlendMode: 'screen',
        opacity: 0.75 * I,
        background: `linear-gradient(
          178deg,
          transparent 0%,
          rgba(255,120,90,0.25) 18%,
          rgba(255,180,90,0.3) 26%,
          rgba(255,235,110,0.32) 34%,
          rgba(150,230,140,0.3) 44%,
          rgba(110,200,240,0.3) 54%,
          rgba(160,140,240,0.28) 66%,
          rgba(220,130,220,0.22) 76%,
          transparent 92%
        )`,
        filter: 'blur(18px)',
      }} />

      {/* Secondary rainbow fragment — smaller, closer to floor */}
      <div style={{
        position: 'absolute', bottom: '-5%', right: '-10%', width: '80%', height: '45%',
        transform: 'rotate(-12deg)',
        mixBlendMode: 'screen',
        opacity: 0.55 * I,
        background: `linear-gradient(
          170deg,
          transparent 0%,
          rgba(255,130,110,0.28) 15%,
          rgba(255,200,100,0.3) 28%,
          rgba(180,230,150,0.28) 45%,
          rgba(130,200,240,0.28) 62%,
          rgba(180,140,240,0.25) 78%,
          transparent 95%
        )`,
        filter: 'blur(22px)',
      }} />

      {/* VOLUMETRIC WARM BEAM — big soft golden shaft across the scene */}
      <div style={{
        position: 'absolute', top: '-30%', left: '-25%', width: '160%', height: '180%',
        transform: 'rotate(22deg)', transformOrigin: 'top left',
        background: 'linear-gradient(90deg, transparent 0%, rgba(255,220,140,0.3) 30%, rgba(255,200,100,0.45) 48%, rgba(255,180,90,0.3) 65%, transparent 92%)',
        mixBlendMode: 'screen',
        opacity: 0.6 * I,
        filter: 'blur(30px)',
      }} />

      {/* THIN GOLDEN RAYS through trees — repeating narrow beams */}
      <div style={{
        position: 'absolute', top: '-30%', left: '-30%', width: '160%', height: '160%',
        transform: 'rotate(22deg)', transformOrigin: 'top left',
        mixBlendMode: 'screen',
        opacity: 0.5 * I,
        background: `repeating-linear-gradient(
          90deg,
          transparent 0,
          transparent 80px,
          rgba(255,230,160,0.25) 80px,
          rgba(255,230,160,0.4) 120px,
          rgba(255,230,160,0.2) 160px,
          transparent 160px,
          transparent 280px
        )`,
        filter: 'blur(8px)',
      }} />

      {/* LENS FLARE ORBS — iridescent bokeh */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', mixBlendMode: 'screen', opacity: 0.8 * I }} preserveAspectRatio="none" viewBox="0 0 100 100">
        <circle cx="62" cy="42" r="3" fill="rgba(255,180,220,0.55)" />
        <circle cx="58" cy="48" r="1.8" fill="rgba(180,220,255,0.5)" />
        <circle cx="75" cy="58" r="2.2" fill="rgba(255,220,160,0.55)" />
        <circle cx="12" cy="68" r="4" fill="rgba(255,140,180,0.45)" />
        <circle cx="85" cy="80" r="1.5" fill="rgba(200,255,200,0.5)" />
        <circle cx="48" cy="75" r="1.2" fill="rgba(255,240,180,0.6)" />
        <circle cx="28" cy="38" r="0.9" fill="rgba(255,255,220,0.8)" />
      </svg>

      {/* Dust motes */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', mixBlendMode: 'screen', opacity: 0.6 * I }}>
        {Array.from({ length: 32 }).map((_, i) => {
          const x = (i * 53) % 100;
          const y = ((i * 31) % 85) + 5;
          const r = 0.6 + (i % 4) * 0.5;
          return <circle key={i} cx={`${x}%`} cy={`${y}%`} r={r} fill="rgba(255,245,200,0.9)" />;
        })}
      </svg>

      {/* WARM TONAL WASH — the whole scene gets saturated with sunset orange/pink */}
      <div style={{
        position: 'absolute', inset: 0,
        background: 'linear-gradient(135deg, rgba(255,180,120,0.18) 0%, rgba(255,150,150,0.12) 50%, rgba(120,80,140,0.15) 100%)',
        mixBlendMode: 'overlay',
        opacity: 0.9 * I,
      }} />

      {/* Deep corner — bottom-right falls into cool shadow for depth (indie film contrast) */}
      <div style={{
        position: 'absolute', bottom: '-10%', right: '-10%', width: '55%', height: '55%',
        background: 'radial-gradient(ellipse, rgba(60,40,80,0.22) 0%, transparent 65%)',
        mixBlendMode: 'multiply',
      }} />

      {/* HEAVY FILM GRAIN — essential for the 90s/indie feel */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.5, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#grain)" fill="black" />
      </svg>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.3, mixBlendMode: 'overlay' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#grainCoarse)" fill="black" />
      </svg>
    </div>
  );
}

// ─── Springy fabric surface — quilted, pressable ───
// Feels soft because: inner gradient (raised), woven texture, stitched edge, pressable spring.
function SoftFabric({ tone = 'butter', radius = 22, padding, style = {}, raised = true, pressable = false, onClick, children }) {
  const tones = {
    paper:    { bg: '#f8e8d4', deep: '#ecc896', hi: '#fff3df', ink: '#3a1810' },
    // indie / 90s-photo jewel tones
    butter:   { bg: '#f5d06a', deep: '#d9a02a', hi: '#ffe59a', ink: '#3a1810' },   // Simpsons yellow
    cream:    { bg: '#eccf94', deep: '#d8b06a', hi: '#f5e0b4', ink: '#4a2610' },
    clay:     { bg: '#e89a80', deep: '#c76e54', hi: '#f3b8a0', ink: '#3a1010' },
    rose:     { bg: '#e78a8a', deep: '#c25a70', hi: '#f6b4b4', ink: '#3a0c20' },    // watermelon
    claret:   { bg: '#9a2a4a', deep: '#5a0e28', hi: '#c24868', ink: '#f8e0d4' },    // deeper wine
    mustard:  { bg: '#e3a423', deep: '#a56a0a', hi: '#f5c857', ink: '#1a0e04' },
    moss:     { bg: '#88b04a', deep: '#5d7828', hi: '#b5d880', ink: '#1e2e0c' },    // lawn green
    pine:     { bg: '#2e6048', deep: '#103a28', hi: '#4e8a6e', ink: '#e8f5dc' },    // fir
    lavender: { bg: '#b49ad8', deep: '#8968b5', hi: '#d4c1ef', ink: '#2a1050' },    // grape
    terra:    { bg: '#d04820', deep: '#8a2a10', hi: '#ee7850', ink: '#fff0d8' },
    pool:     { bg: '#4ca8d8', deep: '#1c6a9a', hi: '#88cdec', ink: '#0a2840' },    // cornflower
    hotpink:  { bg: '#e85a9c', deep: '#a02868', hi: '#f48cc0', ink: '#3a0628' },
    sunset:   { bg: '#f29058', deep: '#c85a28', hi: '#ffb388', ink: '#3a1008' },
  };
  const t = tones[tone] || tones.butter;
  const [pressed, setPressed] = React.useState(false);
  return (
    <div
      onMouseDown={pressable ? () => setPressed(true) : undefined}
      onMouseUp={pressable ? () => setPressed(false) : undefined}
      onMouseLeave={pressable ? () => setPressed(false) : undefined}
      onClick={onClick}
      style={{
        position: 'relative', borderRadius: radius, overflow: 'hidden',
        padding,
        background: raised
          ? `radial-gradient(ellipse 130% 110% at 30% 20%, ${t.hi} 0%, ${t.bg} 45%, ${t.deep} 100%)`
          : t.bg,
        boxShadow: raised
          ? [
              'inset 0 1.5px 0 rgba(255,245,220,0.55)',                // top highlight
              'inset 0 -2px 3px rgba(80,30,20,0.1)',                    // bottom roll
              'inset 0 0 0 0.5px rgba(80,30,20,0.12)',                  // stitch edge
              `0 ${pressed ? 4 : 14}px ${pressed ? 14 : 32}px -10px rgba(${t.ink === '#f0e6d2' || t.ink === '#f5e0c8' || t.ink === '#f5e8d4' ? '40,20,10' : '80,35,20'},${pressed ? 0.18 : 0.32})`,
              '0 3px 8px -4px rgba(80,30,15,0.18)',
            ].join(', ')
          : '0 2px 10px -3px rgba(80,30,15,0.15), inset 0 0 0 0.5px rgba(80,30,20,0.08)',
        transition: 'transform 260ms cubic-bezier(.25,1.6,.5,1), box-shadow 260ms cubic-bezier(.25,1.6,.5,1)',
        transform: pressed ? 'scale(0.97)' : 'scale(1)',
        cursor: pressable ? 'pointer' : 'default',
        ...style,
      }}
    >
      {/* linen weave */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.35, mixBlendMode: 'multiply', pointerEvents: 'none' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#weave)" fill="black" />
      </svg>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.22, mixBlendMode: 'multiply', pointerEvents: 'none' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#weaveH)" fill="black" />
      </svg>
      {/* dashed stitch edge */}
      {raised && (
        <div style={{
          position: 'absolute', inset: 6, borderRadius: Math.max(0, radius - 6),
          border: `1px dashed rgba(${t.ink === '#f0e6d2' || t.ink === '#f5e0c8' || t.ink === '#f5e8d4' ? '240,220,190' : '80,30,15'}, 0.22)`,
          pointerEvents: 'none',
        }} />
      )}
      <div style={{ position: 'relative', zIndex: 2, color: t.ink }}>{children}</div>
    </div>
  );
}

// ─── Liquid / fluted glass — cozy glass ───
function CozyGlass({ tint = 'butter', radius = 24, style = {}, fluted = true, children }) {
  const tints = {
    butter:  'rgba(244, 223, 178, 0.5)',
    rose:    'rgba(216, 154, 150, 0.4)',
    clay:    'rgba(232, 154, 128, 0.38)',
    mustard: 'rgba(215, 154, 62, 0.32)',
    moss:    'rgba(138, 154, 90, 0.28)',
    claret:  'rgba(143, 58, 58, 0.32)',
    clear:   'rgba(252, 238, 208, 0.36)',
  };
  return (
    <div style={{
      position: 'relative', borderRadius: radius, overflow: 'hidden',
      background: tints[tint] || tints.butter,
      backdropFilter: 'blur(22px) saturate(175%)',
      WebkitBackdropFilter: 'blur(22px) saturate(175%)',
      boxShadow: [
        'inset 0 1px 0 rgba(255,245,220,0.55)',
        'inset 0 -1px 0 rgba(90,40,30,0.07)',
        '0 1px 0 0.5px rgba(140,70,50,0.15)',
        '0 10px 30px -12px rgba(90,40,25,0.25)',
      ].join(', '),
      ...style,
    }}>
      {fluted && (
        <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.7, mixBlendMode: 'overlay', pointerEvents: 'none' }}>
          <rect width="100%" height="100%" fill="url(#flute)" />
        </svg>
      )}
      <div style={{ position: 'relative', zIndex: 2 }}>{children}</div>
    </div>
  );
}

// ─── Soft pill button — springy ───
function SoftButton({ tone = 'butter', children, icon, onClick, style = {} }) {
  const [p, setP] = React.useState(false);
  const tones = {
    butter:   { bg: 'linear-gradient(180deg, #faeac5 0%, #e8c378 100%)', ink: '#4a2610' },
    claret:   { bg: 'linear-gradient(180deg, #a85252 0%, #7a2a2a 100%)', ink: '#f5e0c8' },
    rose:     { bg: 'linear-gradient(180deg, #eab8b4 0%, #c48a86 100%)', ink: '#4a1820' },
    clay:     { bg: 'linear-gradient(180deg, #f0a890 0%, #cc6f54 100%)', ink: '#3a1010' },
    moss:     { bg: 'linear-gradient(180deg, #a4b478 0%, #6d7e3f 100%)', ink: '#1e2810' },
    ghost:    { bg: 'rgba(255,245,220,0.4)', ink: '#4a2610' },
  };
  const t = tones[tone] || tones.butter;
  return (
    <button
      onMouseDown={() => setP(true)}
      onMouseUp={() => setP(false)}
      onMouseLeave={() => setP(false)}
      onClick={onClick}
      style={{
        border: 'none',
        display: 'inline-flex', alignItems: 'center', gap: 7,
        padding: '10px 18px', borderRadius: 999,
        background: t.bg, color: t.ink,
        fontFamily: '"SF Pro Rounded", ui-rounded, system-ui, sans-serif',
        fontSize: 14, fontWeight: 600, letterSpacing: 0.1,
        boxShadow: [
          'inset 0 1px 0 rgba(255,245,220,0.55)',
          'inset 0 -1.5px 0 rgba(60,20,10,0.12)',
          `0 ${p ? 2 : 8}px ${p ? 6 : 18}px -4px rgba(80,30,15,0.32)`,
        ].join(', '),
        transform: p ? 'translateY(1px) scale(0.98)' : 'translateY(0) scale(1)',
        transition: 'transform 240ms cubic-bezier(.25,1.7,.4,1), box-shadow 240ms',
        cursor: 'pointer',
        ...style,
      }}
    >
      {icon}
      {children}
    </button>
  );
}

// ─── Rich photo placeholder — saturated 90s camera palette ───
function Photo({ tone, label, ratio = 1, style = {}, mode = 'natural' }) {
  // Saturated jewel-tone swatches — like a 90s disposable camera shot in sunlight
  const tones = {
    butter:  [44, 92, 62],     // hot marigold
    clay:    [14, 88, 60],     // persimmon
    rose:    [345, 78, 66],    // watermelon
    claret:  [350, 72, 42],    // cherry
    mustard: [40, 92, 55],
    moss:    [82, 62, 48],     // lawn
    pine:    [155, 55, 34],    // emerald/fir
    lavender:[270, 58, 64],    // grape
    terra:   [14, 78, 48],
    hemp:    [38, 55, 58],
    sky:     [208, 78, 62],    // cornflower blue
    dusk:    [260, 50, 52],
    pool:    [200, 72, 58],
    hotpink: [330, 75, 62],
    sunset:  [22, 85, 62],
    grape:   [285, 62, 42],
    tangerine:[28, 92, 60],
  };
  const [h, s, l] = tones[tone] || tones.butter;
  // daylight = blown-out warm; otherwise use the saturated color
  const bg = mode === 'daylight'
    ? `linear-gradient(160deg, hsl(${h},${s - 10}%,${l + 15}%), hsl(${Math.max(0, h - 8)},${s}%,${l + 5}%), hsl(${h},${s + 5}%,${l - 15}%))`
    : `linear-gradient(160deg, hsl(${h},${s}%,${l + 10}%), hsl(${h},${s + 5}%,${l}%), hsl(${Math.max(0, h - 15)},${s + 10}%,${l - 15}%))`;
  return (
    <div style={{ position: 'relative', width: '100%', aspectRatio: ratio, overflow: 'hidden', background: bg, ...style }}>
      {/* sun hotspot on the photo */}
      <div style={{ position: 'absolute', inset: 0, background: 'radial-gradient(ellipse 70% 50% at 22% 18%, rgba(255,240,180,0.5), rgba(255,200,140,0.2) 40%, transparent 65%)', mixBlendMode: 'screen' }} />
      {/* prism rainbow sliver on some photos */}
      <div style={{ position: 'absolute', top: '40%', left: 0, right: 0, height: '30%', background: 'linear-gradient(170deg, transparent, rgba(255,140,100,0.18), rgba(255,220,120,0.22), rgba(140,220,160,0.2), rgba(120,180,240,0.2), transparent)', mixBlendMode: 'screen', filter: 'blur(12px)', transform: 'rotate(-6deg)' }} />
      {/* heavy film grain on photos — the 90s look */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.55, mixBlendMode: 'multiply' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#grain)" fill="black" />
      </svg>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.3, mixBlendMode: 'overlay' }} preserveAspectRatio="none">
        <rect width="100%" height="100%" filter="url(#grainCoarse)" fill="black" />
      </svg>
      {/* vignette — like a cheap lens */}
      <div style={{ position: 'absolute', inset: 0, background: 'radial-gradient(ellipse, transparent 50%, rgba(20,5,30,0.35) 100%)', mixBlendMode: 'multiply' }} />
      {label && (
        <div style={{
          position: 'absolute', bottom: 9, left: 11,
          fontFamily: '"SF Pro Rounded", ui-rounded, system-ui, sans-serif',
          fontSize: 10, fontWeight: 600,
          color: 'rgba(255,245,220,0.88)', letterSpacing: 0.6, textTransform: 'uppercase',
          textShadow: '0 1px 2px rgba(40,20,10,0.55)',
        }}>{label}</div>
      )}
    </div>
  );
}

Object.assign(window, { AtmosphereDefs, WindowLight, SoftFabric, CozyGlass, SoftButton, Photo });
