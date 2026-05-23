// Family Room — warmer, more lived-in version.
// Same thesis (people + time as the core), but the tone is friendlier:
// - real-feeling photo placeholders (gradient + content shapes), not flat colored fields
// - rounded corners, casual voice, friendlier type
// - the river is still here, but it's softer and reads like a phone scroll, not a museum wall

const SANS = 'ui-sans-serif, "Söhne", "Inter", -apple-system, system-ui, sans-serif';
const DISPLAY = '"Fraunces", "Source Serif 4", Georgia, serif';
const MONO = '"JetBrains Mono", ui-monospace, monospace';

const PEOPLE = [
  { id: 'M', n: 'Mom',     full: 'Margot',  tone: '#c97a5d', accent: '#f0bfa8' },
  { id: 'D', n: 'Dad',     full: 'Henry',   tone: '#7a6a4a', accent: '#d4c29a' },
  { id: 'C', n: 'Cora',    full: 'Cora',    tone: '#4a7a82', accent: '#a8d0d4' },
  { id: 'T', n: 'Theo',    full: 'Theo',    tone: '#6a5a8a', accent: '#c4b8d8' },
  { id: 'I', n: 'Iris',    full: 'Iris',    tone: '#d49060', accent: '#f5d4b0' },
];

// ─── A friendly photo — a gradient with simple "content" shapes inside.
// Looks more like a real photo than a flat color field, without using actual images.
function Photo({ scene = 'sunset', round = 14, h, w, style = {} }) {
  // a few hand-tuned scenes — each is a gradient + simple shapes
  const scenes = {
    sunset: { // warm sky + landscape silhouette
      bg: 'linear-gradient(180deg, #f5c896 0%, #e89968 35%, #c46e52 70%, #6e4838 100%)',
      content: (
        <>
          <div style={{ position: 'absolute', top: '22%', left: '64%', width: 22, height: 22, borderRadius: '50%',
            background: '#fff5e0', boxShadow: '0 0 22px rgba(255,235,200,0.8)' }}/>
          <div style={{ position: 'absolute', bottom: 0, left: 0, right: 0, height: '38%',
            background: 'linear-gradient(180deg, transparent, rgba(40,25,15,0.55) 60%)'}}/>
          <svg style={{ position: 'absolute', bottom: 0, left: 0, width: '100%', height: '40%' }} viewBox="0 0 100 40" preserveAspectRatio="none">
            <path d="M0 40 L0 22 Q15 16 28 20 Q42 12 58 18 Q72 10 85 16 Q95 12 100 18 L100 40 Z" fill="#3a2418"/>
          </svg>
        </>
      )
    },
    lake: {
      bg: 'linear-gradient(180deg, #b8d4d6 0%, #7ea8a8 50%, #2c4a4f 100%)',
      content: (
        <>
          <div style={{ position: 'absolute', inset: 0, background: 'linear-gradient(180deg, transparent 50%, rgba(255,255,255,0.18) 50%, transparent 55%)' }}/>
          <div style={{ position: 'absolute', top: '38%', left: '20%', width: '60%', height: 1, background: 'rgba(255,255,255,0.4)' }}/>
          <svg style={{ position: 'absolute', bottom: '46%', left: 0, width: '100%', height: '14%' }} viewBox="0 0 100 14" preserveAspectRatio="none">
            <path d="M0 14 L0 8 Q20 4 40 7 Q60 2 80 6 Q90 3 100 5 L100 14 Z" fill="#1a3030"/>
          </svg>
        </>
      )
    },
    indoor: { // cozy living room — warm light from a lamp
      bg: 'linear-gradient(135deg, #6e4f3a 0%, #4a3326 50%, #2a1c14 100%)',
      content: (
        <>
          <div style={{ position: 'absolute', top: '15%', right: '18%', width: 30, height: 30, borderRadius: '50%',
            background: 'radial-gradient(circle, #ffd28a 0%, #e8a060 50%, transparent 70%)', filter: 'blur(2px)' }}/>
          <div style={{ position: 'absolute', top: '10%', right: '15%', width: 36, height: 36, borderRadius: '50%',
            background: 'radial-gradient(circle, rgba(255,210,138,0.4) 0%, transparent 70%)', filter: 'blur(8px)' }}/>
        </>
      )
    },
    cake: {
      bg: 'linear-gradient(180deg, #8a3c3c 0%, #5a2828 100%)',
      content: (
        <>
          {/* candles */}
          <div style={{ position: 'absolute', bottom: '32%', left: '50%', transform: 'translateX(-50%)', display: 'flex', gap: 4 }}>
            {[0,1,2,3,4].map(i => (
              <div key={i} style={{ width: 2, height: 12, background: '#e8c890', position: 'relative' }}>
                <div style={{ position: 'absolute', top: -6, left: -1.5, width: 5, height: 7, borderRadius: '50% 50% 50% 50% / 70% 70% 30% 30%', background: '#ffcc66', boxShadow: '0 0 8px rgba(255,200,100,0.8)' }}/>
              </div>
            ))}
          </div>
          <div style={{ position: 'absolute', bottom: 0, left: '20%', right: '20%', height: '32%', background: '#f0d4b0', borderRadius: '6px 6px 0 0' }}/>
        </>
      )
    },
    garden: {
      bg: 'linear-gradient(180deg, #c4d49a 0%, #7a8a4a 60%, #3a4a2a 100%)',
      content: (
        <>
          <div style={{ position: 'absolute', top: '20%', left: '30%', width: 14, height: 14, borderRadius: '50%', background: '#e8a0a0' }}/>
          <div style={{ position: 'absolute', top: '35%', right: '25%', width: 10, height: 10, borderRadius: '50%', background: '#f0c0c0' }}/>
          <div style={{ position: 'absolute', top: '50%', left: '55%', width: 12, height: 12, borderRadius: '50%', background: '#d488a0' }}/>
        </>
      )
    },
    bike: {
      bg: 'linear-gradient(180deg, #a0c4d4 0%, #6a94a8 50%, #3a5060 100%)',
      content: (
        <svg style={{ position: 'absolute', bottom: '25%', left: '20%', width: '60%' }} viewBox="0 0 60 30">
          <circle cx="12" cy="22" r="6" fill="none" stroke="#1a2030" strokeWidth="1.5"/>
          <circle cx="48" cy="22" r="6" fill="none" stroke="#1a2030" strokeWidth="1.5"/>
          <path d="M12 22 L24 22 L36 8 L48 22 M36 8 L40 22" stroke="#1a2030" strokeWidth="1.5" fill="none"/>
        </svg>
      )
    },
    snow: {
      bg: 'linear-gradient(180deg, #e8eef2 0%, #b8c8d4 60%, #6a7a8a 100%)',
      content: <>{[...Array(20)].map((_,i) => (
        <div key={i} style={{ position: 'absolute', top: `${(i*7)%100}%`, left: `${(i*13)%100}%`,
          width: 2, height: 2, borderRadius: '50%', background: '#fff', opacity: 0.7 }}/>))}
      </>
    },
    stage: {
      bg: 'linear-gradient(180deg, #2a1838 0%, #6a2848 50%, #c44864 100%)',
      content: (
        <>
          <div style={{ position: 'absolute', top: '15%', left: '50%', transform: 'translateX(-50%)', width: 80, height: 60, background: 'radial-gradient(ellipse, rgba(255,210,138,0.5), transparent 70%)', filter: 'blur(8px)' }}/>
          <div style={{ position: 'absolute', bottom: '25%', left: '45%', width: 18, height: 30, borderRadius: '40% 40% 30% 30%', background: '#1a0820' }}/>
        </>
      )
    },
    fall: {
      bg: 'linear-gradient(180deg, #d49060 0%, #b8602c 50%, #6e3818 100%)',
      content: (
        <>
          {[0,1,2,3].map(i => (
            <div key={i} style={{ position: 'absolute',
              top: `${10 + i*22}%`, left: `${20 + (i*23)%60}%`,
              width: 8, height: 8, background: '#e8a050', borderRadius: '0 70% 30% 70%',
              transform: `rotate(${i*40}deg)` }}/>
          ))}
        </>
      )
    },
    wolf: {
      bg: 'linear-gradient(180deg, #1a1028 0%, #2a1838 60%, #4a2848 100%)',
      content: (
        <>
          <div style={{ position: 'absolute', top: '20%', right: '20%', width: 16, height: 16, borderRadius: '50%', background: '#e8d4a0', boxShadow: '0 0 14px #e8d4a0' }}/>
          <svg style={{ position: 'absolute', bottom: 0, left: 0, width: '100%', height: '50%' }} viewBox="0 0 100 50" preserveAspectRatio="none">
            <path d="M0 50 L0 30 L20 26 L30 18 L40 24 L55 16 L70 22 L85 18 L100 24 L100 50 Z" fill="#0a0512"/>
          </svg>
        </>
      )
    },
    table: {
      bg: 'linear-gradient(180deg, #6e4828 0%, #4a2818 100%)',
      content: (
        <>
          <div style={{ position: 'absolute', bottom: '30%', left: '15%', width: 18, height: 18, borderRadius: '50%', background: '#d8a868' }}/>
          <div style={{ position: 'absolute', bottom: '35%', left: '40%', width: 22, height: 22, borderRadius: '50%', background: '#a85838' }}/>
          <div style={{ position: 'absolute', bottom: '32%', right: '20%', width: 16, height: 16, borderRadius: '50%', background: '#e8d090' }}/>
        </>
      )
    },
    studio: {
      bg: 'linear-gradient(135deg, #c8b896 0%, #8a7a5a 100%)',
      content: (
        <>
          <div style={{ position: 'absolute', top: '20%', left: '20%', width: '40%', height: '50%', background: '#fff8ec', boxShadow: '2px 2px 6px rgba(0,0,0,0.15)' }}/>
          <div style={{ position: 'absolute', bottom: '20%', right: '20%', width: 4, height: 30, background: '#3a2818', transform: 'rotate(20deg)' }}/>
        </>
      )
    },
  };
  const s = scenes[scene] || scenes.sunset;
  return (
    <div style={{
      width: w || '100%', height: h || '100%', borderRadius: round, overflow: 'hidden',
      position: 'relative', background: s.bg, ...style,
    }}>
      {s.content}
      {/* slight grain */}
      <div style={{ position: 'absolute', inset: 0, opacity: 0.12, pointerEvents: 'none',
        backgroundImage: 'radial-gradient(circle at 30% 20%, rgba(255,255,255,0.3), transparent 50%)' }}/>
    </div>
  );
}

// ─── Avatar — softer, more "messages app" friendly
function Avatar({ p, size = 36, ring = false, dim = false, showName = false, vert = false }) {
  return (
    <div style={{
      display: 'flex', flexDirection: vert ? 'column' : 'row',
      alignItems: 'center', gap: vert ? 6 : 10,
      opacity: dim ? 0.35 : 1, transition: 'opacity 220ms',
    }}>
      <div style={{
        width: size, height: size, borderRadius: '50%',
        background: `linear-gradient(135deg, ${p.accent} 0%, ${p.tone} 100%)`,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        color: '#fff', fontFamily: SANS, fontWeight: 600,
        fontSize: size * 0.42, letterSpacing: -0.3,
        boxShadow: ring
          ? `0 0 0 2.5px #fff, 0 0 0 4px ${p.tone}`
          : '0 1px 2px rgba(0,0,0,0.12)',
      }}>{p.full[0]}</div>
      {showName && (
        <div style={{
          fontFamily: SANS, fontSize: vert ? 11 : 13.5, fontWeight: 500,
          color: '#3a3228',
        }}>{p.full}</div>
      )}
    </div>
  );
}

// ─── The river: now scrolls horizontally, with friendlier rounded plates & captions
function River({ filtered = null, height = 240 }) {
  // a curated stretch — Mar through Dec
  const items = [
    { d: 'Mar 14', t: "Iris's first bike ride", w: 5, scene: 'bike',  who: ['I','D'] },
    { d: 'Apr 09', t: 'First crocus',           w: 2, scene: 'garden',who: ['M'] },
    { d: 'Apr 22', t: 'Dad came home',          w: 5, scene: 'indoor',who: ['D','M','C'] },
    { d: 'May 05', t: 'Sunday picnic',          w: 3, scene: 'garden',who: ['M','D','C','T','I'] },
    { d: 'Jun 02', t: 'Lake house',             w: 4, scene: 'lake',  who: ['M','D','C','T','I'] },
    { d: 'Jul 04', t: 'Backyard fireworks',     w: 4, scene: 'sunset',who: ['T','I'] },
    { d: 'Aug 09', t: "Dad's 72nd",             w: 5, scene: 'cake',  who: ['M','D','C','T','I'], hero: true },
    { d: 'Sep 22', t: 'Iris on stage',          w: 4, scene: 'stage', who: ['I'] },
    { d: 'Oct 11', t: 'Apple picking',          w: 3, scene: 'fall',  who: ['M','C','I'] },
    { d: 'Oct 31', t: 'Halloween',              w: 4, scene: 'wolf',  who: ['I','T','C'] },
    { d: 'Nov 28', t: 'Thanksgiving',           w: 4, scene: 'table', who: ['M','D','C','T','I'] },
    { d: 'Dec 24', t: 'Christmas Eve',          w: 5, scene: 'indoor',who: ['M','D','C','T','I'] },
  ];
  const sizeOf = w => ({ width: [80, 110, 150, 190, 230][w-1], height: [70, 95, 130, 165, 200][w-1] });

  return (
    <div style={{ display: 'flex', alignItems: 'flex-end', gap: 14, padding: '20px 4px 0',
      overflowX: 'auto', minHeight: height }}>
      {items.map((m, i) => {
        const active = !filtered || m.who.some(id => filtered.includes(id));
        const sz = sizeOf(m.w);
        return (
          <div key={i} style={{
            flexShrink: 0, opacity: active ? 1 : 0.25, transition: 'opacity 280ms',
            position: 'relative',
          }}>
            <Photo scene={m.scene} h={sz.height} w={sz.width} round={10} />
            <div style={{
              marginTop: 10,
              fontFamily: SANS, fontSize: 12.5, fontWeight: 600, color: '#2a221a',
              letterSpacing: -0.1, lineHeight: 1.25,
            }}>{m.t}</div>
            <div style={{
              fontFamily: SANS, fontSize: 11, fontWeight: 500, color: '#a89c8c', marginTop: 2,
            }}>{m.d}{m.hero && ' · everyone'}</div>
          </div>
        );
      })}
    </div>
  );
}

// ═══════════════════════════════════════════════════════
// Mac
// ═══════════════════════════════════════════════════════
const NEU_OUT    = '-8px -8px 18px rgba(255,250,238,0.95), 8px 8px 18px rgba(120,95,60,0.30)';
const NEU_OUT_SM = '-4px -4px 9px rgba(255,250,238,0.95), 4px 4px 9px rgba(120,95,60,0.28)';
const NEU_IN     = 'inset 4px 4px 8px rgba(120,95,60,0.32), inset -4px -4px 8px rgba(255,250,238,0.95)';
const NEU_IN_SM  = 'inset 2px 2px 5px rgba(120,95,60,0.30), inset -2px -2px 5px rgba(255,250,238,0.92)';
const CREAM = '#ece4d6';

function MacChrome({ width = 1280, height = 820, title, children }) {
  return (
    <div style={{
      width, height, flexShrink: 0,
      borderRadius: 28, overflow: 'hidden', position: 'relative',
      background: CREAM,
      boxShadow: NEU_OUT + ', 0 40px 80px -30px rgba(40,30,20,0.25)',
      display: 'flex', flexDirection: 'column', fontFamily: SANS,
    }}>
      <div style={{ height: 40, flexShrink: 0, display: 'flex', alignItems: 'center',
        padding: '0 16px', background: CREAM,
        boxShadow: 'inset 0 -1px 0 rgba(120,95,60,0.10)' }}>
        <div style={{ display: 'flex', gap: 9 }}>
          {['#e07a6a','#d8a85a','#7ea878'].map((c, i) => (
            <div key={i} style={{ width: 13, height: 13, borderRadius: 7, background: c,
              boxShadow: 'inset 1.5px 1.5px 2.5px rgba(0,0,0,0.18), inset -1px -1px 2px rgba(255,255,255,0.45), 0 1px 1px rgba(255,250,238,0.6)' }}/>
          ))}
        </div>
        <div style={{ flex: 1, textAlign: 'center', fontFamily: SANS,
          fontSize: 12.5, fontWeight: 500, color: '#5a4f42' }}>{title}</div>
      </div>
      {children}
    </div>
  );
}

function MacMain() {
  return (
    <MacChrome title="Family Room">
      <div style={{ flex: 1, display: 'flex', minHeight: 0, background: CREAM, padding: 18, gap: 18 }}>
        {/* Left rail */}
        <div style={{ width: 220, flexShrink: 0, padding: '20px 16px',
          borderRadius: 22, background: CREAM, boxShadow: NEU_IN,
          display: 'flex', flexDirection: 'column', gap: 14 }}>
          <div style={{ fontFamily: SANS, fontSize: 11, fontWeight: 600, letterSpacing: 1,
            textTransform: 'uppercase', color: '#a89c8c' }}>Family</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {PEOPLE.map((p,i) => (
              <div key={p.id} style={{
                display: 'flex', alignItems: 'center', gap: 12, padding: '10px 12px',
                borderRadius: 14, background: CREAM,
                boxShadow: i === 2 ? NEU_OUT_SM : 'none',
                cursor: 'pointer',
              }}>
                <Avatar p={p} size={32} ring={i === 2}/>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600, color: '#2a221a' }}>{p.full}</div>
                  <div style={{ fontSize: 11, color: '#a89c8c' }}>{p.id === 'M' ? '892 photos' : p.id === 'D' ? '654 photos' : p.id === 'C' ? '1,204 photos' : p.id === 'T' ? '743 photos' : '2,108 photos'}</div>
                </div>
              </div>
            ))}
          </div>

          <div style={{ height: 0.5, background: 'rgba(40,30,20,0.08)', margin: '8px 0' }}/>

          <div style={{ fontFamily: SANS, fontSize: 11, fontWeight: 600, letterSpacing: 1,
            textTransform: 'uppercase', color: '#a89c8c' }}>Places</div>
          {['The lake house', 'Margot & Henry\'s', 'Our backyard', 'Iris\'s school'].map((p,i) => (
            <div key={i} style={{ fontSize: 13, color: '#5a4f42', padding: '4px 10px', cursor: 'pointer' }}>{p}</div>
          ))}

          <div style={{ flex: 1 }}/>

          <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '10px 14px',
            borderRadius: 14, background: CREAM, boxShadow: NEU_IN_SM }}>
            <div style={{ width: 8, height: 8, borderRadius: '50%', background: '#7ea878',
              boxShadow: '0 0 6px rgba(126,168,120,0.6), inset 0 0 2px rgba(255,255,255,0.5)' }}/>
            <div style={{ fontSize: 12, color: '#5a4f42' }}>4 of 5 syncing</div>
          </div>
        </div>

        {/* Main */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0,
          overflow: 'hidden', borderRadius: 22, background: CREAM, padding: '4px 6px' }}>
          {/* Search */}
          <div style={{ padding: '14px 18px 14px', display: 'flex', gap: 12, alignItems: 'center' }}>
            <div style={{ flex: 1, position: 'relative' }}>
              <input defaultValue="Cora — anything from this year"
                style={{ width: '100%', boxSizing: 'border-box',
                  padding: '12px 18px 12px 42px', borderRadius: 14,
                  border: 'none', background: CREAM, boxShadow: NEU_IN_SM,
                  fontFamily: SANS, fontSize: 14, color: '#2a221a', outline: 'none' }}/>
              <svg style={{ position: 'absolute', left: 14, top: '50%', transform: 'translateY(-50%)' }}
                width="14" height="14" viewBox="0 0 14 14">
                <circle cx="6" cy="6" r="4.5" fill="none" stroke="#a89c8c" strokeWidth="1.5"/>
                <path d="M9.5 9.5 L13 13" stroke="#a89c8c" strokeWidth="1.5" strokeLinecap="round"/>
              </svg>
            </div>
            <button style={{ padding: '12px 18px', borderRadius: 14, border: 'none',
              background: CREAM, color: '#c97a5d', fontFamily: SANS, fontSize: 13,
              fontWeight: 600, cursor: 'pointer', boxShadow: NEU_OUT_SM }}>+ Add photos</button>
          </div>

          {/* Hero */}
          <div style={{ padding: '24px 32px 8px' }}>
            <div style={{ fontFamily: SANS, fontSize: 12, fontWeight: 600,
              letterSpacing: 0.6, textTransform: 'uppercase', color: '#c97a5d' }}>This year, with Cora</div>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 14, marginTop: 4 }}>
              <div style={{ fontFamily: DISPLAY, fontSize: 44, fontWeight: 500,
                lineHeight: 1, color: '#2a221a', letterSpacing: -1 }}>2024</div>
              <div style={{ fontSize: 15, color: '#7a6a5a' }}>14 moments · 326 photos</div>
            </div>
          </div>

          {/* River */}
          <div style={{ padding: '0 32px', flex: 1, minHeight: 0, overflow: 'hidden' }}>
            <River filtered={['C']} />
          </div>

          {/* Recently shared section */}
          <div style={{ padding: '8px 32px 20px' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
              <div style={{ fontFamily: DISPLAY, fontSize: 20, fontWeight: 500, color: '#2a221a', letterSpacing: -0.3 }}>
                Shared this week
              </div>
              <div style={{ fontSize: 12, color: '#a89c8c' }}>see all →</div>
            </div>
            <div style={{ display: 'flex', gap: 14 }}>
              {[
                { p: 'I', scene: 'bike',   t: 'Iris added 4 photos', d: '2 hours ago' },
                { p: 'M', scene: 'garden', t: 'Margot added a video', d: 'yesterday' },
                { p: 'T', scene: 'studio', t: 'Theo shared a sketch', d: '3 days ago' },
              ].map((s,i) => {
                const person = PEOPLE.find(p => p.id === s.p);
                return (
                  <div key={i} style={{ flex: 1, display: 'flex', gap: 12, padding: 12,
                    background: '#fff', borderRadius: 12, border: '0.5px solid rgba(40,30,20,0.08)' }}>
                    <Photo scene={s.scene} w={60} h={60} round={8}/>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                        <Avatar p={person} size={16}/>
                        <div style={{ fontSize: 12, fontWeight: 600, color: '#2a221a' }}>{s.t}</div>
                      </div>
                      <div style={{ fontSize: 11, color: '#a89c8c' }}>{s.d}</div>
                      <div style={{ display: 'flex', gap: 12, marginTop: 6, fontSize: 11, color: '#7a6a5a' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}><window.Icon name="heart" size={11} color="#7a6a5a"/>4</div>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}><window.Icon name="comment" size={11} color="#7a6a5a"/>2</div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </MacChrome>
  );
}

// ═══════════════════════════════════════════════════════
// iPhone
// ═══════════════════════════════════════════════════════
function PhoneFrame({ children, dark = false, width = 390, height = 844 }) {
  return (
    <div style={{
      width, height, borderRadius: 50, position: 'relative', flexShrink: 0,
      background: dark ? '#000' : CREAM, overflow: 'hidden',
      boxShadow: '0 0 0 9px #1a1612, 0 0 0 10px #2a221a, ' + NEU_OUT + ', 0 40px 80px -25px rgba(40,30,20,0.45)',
      fontFamily: SANS,
    }}>
      {/* notch / dynamic island */}
      <div style={{ position: 'absolute', top: 11, left: '50%', transform: 'translateX(-50%)',
        width: 120, height: 36, borderRadius: 22, background: '#000', zIndex: 50 }}/>
      {/* status */}
      <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 54, zIndex: 10,
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '18px 30px 0' }}>
        <div style={{ fontSize: 15, fontWeight: 600, color: dark ? '#fff' : '#000' }}>9:41</div>
        <div style={{ display: 'flex', gap: 5, alignItems: 'center' }}>
          <svg width="16" height="11" viewBox="0 0 16 11"><rect x="0" y="7" width="3" height="4" rx="0.5" fill={dark ? '#fff' : '#000'}/><rect x="4.5" y="5" width="3" height="6" rx="0.5" fill={dark ? '#fff' : '#000'}/><rect x="9" y="2.5" width="3" height="8.5" rx="0.5" fill={dark ? '#fff' : '#000'}/><rect x="13.5" y="0" width="3" height="11" rx="0.5" fill={dark ? '#fff' : '#000'}/></svg>
          <svg width="22" height="11" viewBox="0 0 22 11"><rect x="0.5" y="0.5" width="19" height="10" rx="2.5" stroke={dark ? '#fff' : '#000'} strokeOpacity="0.4" fill="none"/><rect x="2" y="2" width="16" height="7" rx="1.5" fill={dark ? '#fff' : '#000'}/></svg>
        </div>
      </div>
      <div style={{ position: 'absolute', inset: 0, paddingTop: 54 }}>{children}</div>
      {/* home indicator */}
      <div style={{ position: 'absolute', bottom: 8, left: '50%', transform: 'translateX(-50%)',
        width: 134, height: 5, borderRadius: 3, background: dark ? 'rgba(255,255,255,0.4)' : 'rgba(0,0,0,0.3)' }}/>
    </div>
  );
}

function PhoneFeed() {
  return (
    <PhoneFrame>
      <div style={{ height: '100%', overflow: 'auto', paddingBottom: 100 }}>
        {/* Header */}
        <div style={{ padding: '8px 24px 12px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div>
            <div style={{ fontSize: 13, color: '#a89c8c' }}>Hi Cora —</div>
            <div style={{ fontFamily: DISPLAY, fontSize: 28, fontWeight: 500, color: '#2a221a', letterSpacing: -0.5, marginTop: 2 }}>
              Family
            </div>
          </div>
          <div style={{ width: 44, height: 44, borderRadius: 22, background: CREAM,
            boxShadow: NEU_OUT_SM, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <svg width="18" height="18" viewBox="0 0 18 18">
              <circle cx="7.5" cy="7.5" r="6" fill="none" stroke="#5a4f42" strokeWidth="1.5"/>
              <path d="M12 12 L16 16" stroke="#5a4f42" strokeWidth="1.5" strokeLinecap="round"/>
            </svg>
          </div>
        </div>

        {/* People row */}
        <div style={{ padding: '0 24px 16px', display: 'flex', gap: 14, overflow: 'auto' }}>
          {PEOPLE.map(p => (
            <div key={p.id} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, flexShrink: 0 }}>
              <div style={{ position: 'relative' }}>
                <Avatar p={p} size={56} />
                <div style={{ position: 'absolute', bottom: -2, right: -2, width: 16, height: 16,
                  borderRadius: 8, background: '#7ea878', boxShadow: '0 0 0 2.5px ' + CREAM + ', 0 0 6px rgba(126,168,120,0.5)',
                  display: p.id === 'M' || p.id === 'I' ? 'block' : 'none' }}/>
              </div>
              <div style={{ fontSize: 11.5, fontWeight: 500, color: '#2a221a' }}>{p.full}</div>
            </div>
          ))}
        </div>

        {/* Today section */}
        <div style={{ padding: '0 24px 8px' }}>
          <div style={{ fontFamily: DISPLAY, fontSize: 17, fontWeight: 500, color: '#2a221a', letterSpacing: -0.2 }}>
            Today
          </div>
        </div>

        {/* Hero card */}
        <div style={{ padding: '8px 24px 14px' }}>
          <div style={{ borderRadius: 22, overflow: 'hidden', background: CREAM,
            boxShadow: NEU_OUT_SM, padding: 4 }}>
            <Photo scene="bike" h={220} round={0}/>
            <div style={{ padding: '12px 14px 14px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                <Avatar p={PEOPLE[4]} size={20}/>
                <div style={{ fontSize: 12, fontWeight: 600, color: '#2a221a' }}>Iris · 2 hours ago</div>
              </div>
              <div style={{ fontFamily: DISPLAY, fontSize: 17, fontWeight: 500, color: '#2a221a',
                letterSpacing: -0.2, lineHeight: 1.25 }}>
                Iris's first bike ride
              </div>
              <div style={{ fontSize: 13, color: '#7a6a5a', marginTop: 4, lineHeight: 1.4 }}>
                "She didn't even ask for the training wheels." — Theo
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 14, marginTop: 12,
                paddingTop: 12, borderTop: '0.5px solid rgba(40,30,20,0.08)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 5, color: '#c97a5d', fontSize: 13, fontWeight: 600 }}>
                  <window.Icon name="heart" size={14} color="#c97a5d"/> 4
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 5, fontSize: 13, color: '#7a6a5a' }}>
                  <window.Icon name="comment" size={14} color="#7a6a5a"/> 2 comments
                </div>
                <div style={{ flex: 1 }}/>
                <div style={{ fontSize: 12, color: '#a89c8c' }}>4 photos</div>
              </div>
            </div>
          </div>
        </div>

        {/* On this day */}
        <div style={{ padding: '6px 24px 12px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
            <div style={{ fontFamily: DISPLAY, fontSize: 17, fontWeight: 500, color: '#2a221a', letterSpacing: -0.2 }}>
              On this day
            </div>
            <div style={{ fontSize: 12, color: '#a89c8c' }}>3 years</div>
          </div>
          <div style={{ display: 'flex', gap: 10 }}>
            {[
              { y: '2023', s: 'garden' },
              { y: '2022', s: 'lake' },
              { y: '2020', s: 'studio' },
            ].map((d,i) => (
              <div key={i} style={{ flex: 1, position: 'relative' }}>
                <Photo scene={d.s} h={94} round={12}/>
                <div style={{ position: 'absolute', bottom: 6, left: 8,
                  fontFamily: SANS, fontSize: 11, fontWeight: 600, color: '#fff',
                  textShadow: '0 1px 2px rgba(0,0,0,0.4)' }}>{d.y}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Earlier */}
        <div style={{ padding: '6px 24px 0' }}>
          <div style={{ fontFamily: DISPLAY, fontSize: 17, fontWeight: 500, color: '#2a221a', letterSpacing: -0.2, marginBottom: 10 }}>
            Earlier this week
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {[
              { p: 'M', scene: 'garden', t: 'Margot', m: 'Roses are coming back', d: 'Mon' },
              { p: 'T', scene: 'studio', t: 'Theo',   m: 'New piece in progress', d: 'Sun' },
            ].map((s,i) => {
              const p = PEOPLE.find(x => x.id === s.p);
              return (
                <div key={i} style={{ display: 'flex', gap: 12, alignItems: 'center',
                  padding: 12, background: CREAM, borderRadius: 18,
                  boxShadow: NEU_OUT_SM }}>
                  <Photo scene={s.scene} w={56} h={56} round={10}/>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>{s.t}</div>
                    <div style={{ fontSize: 12, color: '#7a6a5a', marginTop: 2 }}>{s.m}</div>
                  </div>
                  <div style={{ fontSize: 11, color: '#a89c8c' }}>{s.d}</div>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* Tab bar */}
      <nav aria-label="Sections" style={{ position: 'absolute', bottom: 0, left: 0, right: 0, height: 86,
        background: CREAM, boxShadow: 'inset 0 1px 0 rgba(255,250,238,0.6), 0 -8px 18px rgba(120,95,60,0.10)',
        display: 'flex', justifyContent: 'space-around', alignItems: 'flex-start',
        paddingTop: 10 }}>
        {[
          { i: 'home',   l: 'Home', on: true },
          { i: 'people', l: 'People' },
          { i: 'plus',   l: 'Add', plus: true },
          { i: 'film',   l: 'Memories' },
          { i: 'cog',    l: 'You' },
        ].map((t,i) => (
          <button key={i} aria-label={t.l} aria-current={t.on ? 'page' : undefined}
            style={{ background: 'transparent', border: 'none', cursor: 'pointer',
              display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6,
              padding: '4px 8px', minWidth: 56, minHeight: 48 }}>
            <div style={{ width: t.plus ? 38 : 30, height: t.plus ? 38 : 30, borderRadius: t.plus ? 12 : 8,
              background: t.plus ? 'linear-gradient(135deg, #f0bfa8, #b86348)' : CREAM,
              boxShadow: t.plus ? '0 4px 10px rgba(184,99,72,0.35)' : (t.on ? NEU_IN_SM : 'none'),
              display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <window.Icon name={t.i} size={t.plus ? 20 : 18}
                color={t.plus ? '#fff' : (t.on ? '#b86348' : '#6e6358')}
                strokeWidth={t.plus ? 2.2 : 1.8}/>
            </div>
            <div style={{ fontSize: 10.5, fontWeight: 600,
              color: t.on ? '#2a221a' : '#6e6358' }}>{t.l}</div>
          </button>
        ))}
      </nav>
    </PhoneFrame>
  );
}

function PhoneMoment() {
  return (
    <PhoneFrame>
      <div style={{ height: '100%', overflow: 'auto', paddingBottom: 100 }}>
        {/* Back nav */}
        <div style={{ padding: '8px 18px 8px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 4, color: '#c97a5d', fontSize: 15, fontWeight: 500 }}>
            ‹ Family
          </div>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: 32, height: 32 }}><window.Icon name="more" size={18} color="#5a4f42"/></div>
        </div>

        {/* Big hero */}
        <div style={{ padding: '4px 18px 16px' }}>
          <Photo scene="cake" h={300} round={20}/>
        </div>

        <div style={{ padding: '0 24px' }}>
          <div style={{ fontFamily: SANS, fontSize: 11, fontWeight: 600, letterSpacing: 0.6,
            textTransform: 'uppercase', color: '#c97a5d' }}>Aug 9 · 2024 · The Lake House</div>
          <div style={{ fontFamily: DISPLAY, fontSize: 30, fontWeight: 500, color: '#2a221a',
            letterSpacing: -0.5, marginTop: 4, lineHeight: 1.1 }}>
            Dad's 72nd
          </div>
          <div style={{ fontSize: 14.5, color: '#5a4f42', marginTop: 8, lineHeight: 1.45,
            fontFamily: DISPLAY, fontStyle: 'italic' }}>
            We took the boat out at six. The sky was the color of a peach pit. Iris fell asleep on his shoulder.
          </div>

          {/* Who was there */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 16,
            padding: '12px 14px', background: '#fff', borderRadius: 12,
            border: '0.5px solid rgba(40,30,20,0.06)' }}>
            <div style={{ display: 'flex', marginLeft: 0 }}>
              {PEOPLE.map((p,i) => (
                <div key={p.id} style={{ marginLeft: i === 0 ? 0 : -8 }}>
                  <Avatar p={p} size={28}/>
                </div>
              ))}
            </div>
            <div style={{ flex: 1, fontSize: 12.5, color: '#5a4f42' }}>Everyone was here</div>
          </div>

          {/* Photos grid */}
          <div style={{ marginTop: 20, display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 6 }}>
            <div style={{ gridRow: 'span 2' }}><Photo scene="sunset" h={186} round={8}/></div>
            <Photo scene="lake" h={90} round={8}/>
            <Photo scene="indoor" h={90} round={8}/>
          </div>
          <div style={{ marginTop: 6, display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 6 }}>
            <Photo scene="cake" h={70} round={8}/>
            <Photo scene="garden" h={70} round={8}/>
            <Photo scene="indoor" h={70} round={8}/>
          </div>

          {/* Stats */}
          <div style={{ display: 'flex', gap: 8, marginTop: 16 }}>
            {[
              { n: '61', l: 'photos' },
              { n: '3', l: 'videos' },
              { n: '5.0', l: 'family weight' },
            ].map((s,i) => (
              <div key={i} style={{ flex: 1, padding: 10, background: '#fff',
                borderRadius: 10, border: '0.5px solid rgba(40,30,20,0.06)', textAlign: 'center' }}>
                <div style={{ fontFamily: DISPLAY, fontSize: 20, fontWeight: 500, color: '#2a221a' }}>{s.n}</div>
                <div style={{ fontSize: 10.5, color: '#a89c8c', textTransform: 'uppercase', letterSpacing: 0.6, marginTop: 1 }}>{s.l}</div>
              </div>
            ))}
          </div>

          {/* Comments */}
          <div style={{ marginTop: 18 }}>
            <div style={{ fontFamily: DISPLAY, fontSize: 16, fontWeight: 500, color: '#2a221a',
              letterSpacing: -0.2, marginBottom: 10 }}>
              What everyone said
            </div>
            {[
              { p: 'I', t: 'best day of my life', d: '6:14 pm' },
              { p: 'M', t: 'so happy with my whole family', d: '7:02 pm' },
            ].map((c,i) => {
              const p = PEOPLE.find(x => x.id === c.p);
              return (
                <div key={i} style={{ display: 'flex', gap: 10, marginBottom: 10 }}>
                  <Avatar p={p} size={28}/>
                  <div style={{ flex: 1, padding: '8px 12px', background: '#fff',
                    borderRadius: 12, border: '0.5px solid rgba(40,30,20,0.06)' }}>
                    <div style={{ fontSize: 12, fontWeight: 600, color: '#2a221a', marginBottom: 2 }}>{p.full}</div>
                    <div style={{ fontSize: 13, color: '#3a3228', lineHeight: 1.35 }}>{c.t}</div>
                    <div style={{ fontSize: 10.5, color: '#a89c8c', marginTop: 4 }}>{c.d}</div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </PhoneFrame>
  );
}

// ═══════════════════════════════════════════════════════
// iPad — landscape, designed for the living room
// ═══════════════════════════════════════════════════════
function IPadFrame({ children, width = 1180, height = 820 }) {
  return (
    <div style={{
      width: width + 36, height: height + 36, borderRadius: 38,
      background: '#1a1612', padding: 18, flexShrink: 0,
      boxShadow: NEU_OUT + ', 0 40px 80px -25px rgba(40,30,20,0.45)',
      position: 'relative',
    }}>
      <div style={{ width, height, borderRadius: 22, overflow: 'hidden',
        background: CREAM, position: 'relative', fontFamily: SANS }}>
        {/* status bar */}
        <div style={{ height: 28, display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          padding: '8px 22px', fontSize: 12, color: '#2a221a', fontWeight: 600 }}>
          <div style={{ display: 'flex', gap: 14 }}>
            <span>9:41</span>
          </div>
          <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
            <span style={{ fontSize: 11 }}>The Alderhouse</span>
            <svg width="16" height="11" viewBox="0 0 16 11"><rect x="0" y="7" width="3" height="4" rx="0.5" fill="#000"/><rect x="4.5" y="5" width="3" height="6" rx="0.5" fill="#000"/><rect x="9" y="2.5" width="3" height="8.5" rx="0.5" fill="#000"/><rect x="13.5" y="0" width="3" height="11" rx="0.5" fill="#000"/></svg>
            <svg width="22" height="11" viewBox="0 0 22 11"><rect x="0.5" y="0.5" width="19" height="10" rx="2.5" stroke="#000" strokeOpacity="0.4" fill="none"/><rect x="2" y="2" width="16" height="7" rx="1.5" fill="#000"/></svg>
          </div>
        </div>
        {children}
      </div>
      {/* camera */}
      <div style={{ position: 'absolute', top: '50%', right: 8, transform: 'translateY(-50%)',
        width: 6, height: 6, borderRadius: 3, background: '#3a3228' }}/>
    </div>
  );
}

function IPadLanding() {
  return (
    <IPadFrame>
      <div style={{ display: 'flex', height: 'calc(100% - 28px)' }}>
        {/* Sidebar */}
        <div style={{ width: 240, padding: '8px 16px 20px', display: 'flex', flexDirection: 'column', gap: 14 }}>
          <div style={{ fontFamily: DISPLAY, fontSize: 22, fontWeight: 500, color: '#2a221a',
            letterSpacing: -0.4, padding: '0 6px' }}>
            Family
          </div>
          <div style={{ fontFamily: SANS, fontSize: 10.5, fontWeight: 600, letterSpacing: 1,
            textTransform: 'uppercase', color: '#a89c8c', padding: '0 6px' }}>People</div>
          {PEOPLE.map((p,i) => (
            <div key={p.id} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '8px 10px',
              borderRadius: 12, background: CREAM,
              boxShadow: i === 4 ? NEU_OUT_SM : 'none' }}>
              <Avatar p={p} size={28} ring={i === 4}/>
              <div style={{ flex: 1, fontSize: 13, fontWeight: 500, color: '#2a221a' }}>{p.full}</div>
              {i === 4 && <div style={{ fontSize: 10, color: '#d49060', fontWeight: 600 }}>2.1k</div>}
            </div>
          ))}

          <div style={{ height: 0.5, background: 'rgba(40,30,20,0.08)', margin: '4px 0' }}/>

          <div style={{ fontFamily: SANS, fontSize: 10.5, fontWeight: 600, letterSpacing: 1,
            textTransform: 'uppercase', color: '#a89c8c', padding: '0 6px' }}>Years</div>
          {['2024', '2023', '2022', '2021', '2020', 'Older'].map((y, i) => (
            <div key={y} style={{ padding: '6px 6px', fontSize: 13, color: i === 0 ? '#2a221a' : '#5a4f42',
              fontWeight: i === 0 ? 600 : 400 }}>{y}</div>
          ))}
        </div>

        {/* Main canvas — Iris's view */}
        <div style={{ flex: 1, padding: '4px 22px 18px', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', padding: '8px 0 14px' }}>
            <div>
              <div style={{ fontSize: 12, fontWeight: 600, color: '#d49060', textTransform: 'uppercase', letterSpacing: 0.6 }}>
                With Iris · 2024
              </div>
              <div style={{ fontFamily: DISPLAY, fontSize: 32, fontWeight: 500, color: '#2a221a',
                letterSpacing: -0.6, marginTop: 2 }}>
                Her year, big and small
              </div>
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              <button style={{ padding: '10px 16px', borderRadius: 12, border: 'none',
                background: CREAM, fontSize: 12.5, fontWeight: 500, color: '#2a221a', boxShadow: NEU_OUT_SM }}>Slideshow</button>
              <button style={{ padding: '10px 16px', borderRadius: 12, border: 'none',
                background: CREAM, fontSize: 12.5, fontWeight: 600, color: '#d49060', boxShadow: NEU_OUT_SM }}>+ Add</button>
            </div>
          </div>

          {/* The river */}
          <div style={{ flex: 1, minHeight: 0, overflow: 'hidden' }}>
            <River filtered={['I']} height={280}/>
          </div>

          {/* Bottom row — Iris's scrapbook */}
          <div style={{ marginTop: 12, padding: '16px 18px', background: CREAM, borderRadius: 20,
            boxShadow: NEU_OUT_SM }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <Avatar p={PEOPLE[4]} size={28}/>
                <div>
                  <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>Iris's scrapbook</div>
                  <div style={{ fontSize: 11, color: '#a89c8c' }}>she made this · last edit yesterday</div>
                </div>
              </div>
              <div style={{ fontSize: 12, color: '#a89c8c' }}>9 pages →</div>
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              {['bike','garden','stage','wolf','snow','fall','lake','sunset','cake'].map((s,i) => (
                <div key={i} style={{ flex: 1, transform: `rotate(${(i % 2 ? 1 : -1) * (1 + i % 3)}deg)` }}>
                  <Photo scene={s} h={70} round={3} style={{ boxShadow: '0 1px 3px rgba(40,30,20,0.15)', border: '4px solid #fff' }}/>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </IPadFrame>
  );
}

window.MacMain = MacMain;
window.PhoneFeed = PhoneFeed;
window.PhoneMoment = PhoneMoment;
window.IPadLanding = IPadLanding;
window.Photo = Photo;
window.Avatar = Avatar;
