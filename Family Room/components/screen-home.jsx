// Family Room — Screen compositions
// All screens composed as content that drops into an iPhone frame (402 × 874).
// Warm, tactile, sunlight aesthetic. No emoji. Placeholders for imagery.

const FAMILY_NAME = "The Alderhouse";
const MEMBERS = [
  { n: 'Margot',    r: 'Grandma',  hue: 20,  tone: 'rose'  },
  { n: 'Henry',     r: 'Grandpa',  hue: 35,  tone: 'oat'   },
  { n: 'Clara',     r: 'Mom',      hue: 12,  tone: 'rose'  },
  { n: 'Theo',      r: 'Dad',      hue: 30,  tone: 'stone' },
  { n: 'Iris',      r: 'Daughter', hue: 340, tone: 'rose'  },
  { n: 'Jun',       r: 'Son',      hue: 200, tone: 'sage'  },
  { n: 'Lena',      r: 'Aunt',     hue: 45,  tone: 'oat'   },
  { n: 'Marcus',    r: 'Uncle',    hue: 15,  tone: 'stone' },
];

function Avatar({ name, hue = 30, size = 40, ring = false }) {
  const initial = name?.[0] || '?';
  return (
    <div style={{
      width: size, height: size, borderRadius: '50%', position: 'relative',
      background: `radial-gradient(circle at 30% 30%, hsl(${hue},55%,80%), hsl(${hue},45%,58%))`,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      color: '#fff', fontFamily: '"Fraunces", Georgia, serif',
      fontWeight: 500, fontSize: size * 0.42,
      boxShadow: ring
        ? `0 0 0 2px #fff8e8, 0 0 0 4px hsl(${hue},50%,55%), 0 2px 6px rgba(0,0,0,0.15)`
        : '0 1px 3px rgba(0,0,0,0.18), inset 0 -2px 6px rgba(0,0,0,0.12)',
      flexShrink: 0,
    }}>
      {initial}
    </div>
  );
}

// ─── Wrapper applied once for every phone screen ────────────────
function ScreenShell({ children, warmth = 'afternoon', intensity = 1, bg = '#f7ecd5', showStatus = true, dark = false }) {
  return (
    <div style={{
      position: 'relative', width: '100%', height: '100%',
      background: dark ? '#1a1410' : bg,
      overflow: 'hidden',
    }}>
      <SunlightWash warmth={warmth} intensity={intensity} />
      <div style={{ position: 'relative', zIndex: 2, height: '100%', overflow: 'auto' }}>
        {children}
      </div>
    </div>
  );
}

// Status-bar-safe header wrapper (leaves space for dynamic island)
function HeaderPad({ children }) {
  return <div style={{ paddingTop: 58 }}>{children}</div>;
}

// ═══════════════════════════════════════════════════════════════
// SCREEN 1 — HOME / FAMILY ROOM
// A "mantlepiece" shelf at top with family members, wood surface.
// Below it: a warm feed of recent moments with soft paper-edged tiles.
// ═══════════════════════════════════════════════════════════════
function HomeScreen({ woodTone = 'walnut', warmth = 'afternoon', intensity = 1, density = 'balanced' }) {
  const cols = density === 'dense' ? 3 : density === 'airy' ? 1 : 2;
  const moments = [
    { t: "Sunday lunch",        s: "Margot's kitchen · 24 photos",  hue: 28, by: 'Clara',  ago: 'today' },
    { t: "Iris learns to ride", s: "the back road · 1 video",       hue: 180, by: 'Theo', ago: 'yesterday' },
    { t: "Henry's 72nd",        s: "lake house · 61 photos",        hue: 40, by: 'Margot', ago: '3 days ago' },
    { t: "First tomatoes",      s: "garden · 7 photos",             hue: 100, by: 'Lena', ago: 'last week' },
  ];

  return (
    <ScreenShell warmth={warmth} intensity={intensity}>
      <HeaderPad>
        {/* top bar */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '6px 20px 0' }}>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 13, letterSpacing: 2, color: '#7a5a3a', textTransform: 'uppercase' }}>
            Family Room
          </div>
          <div style={{ display: 'flex', gap: 10, color: '#5a3a1e' }}>
            <div style={{ width: 34, height: 34, borderRadius: 17, background: 'rgba(255,255,255,0.55)', display: 'flex', alignItems: 'center', justifyContent: 'center', backdropFilter: 'blur(8px)' }}>
              {Icon.search(18, '#5a3a1e')}
            </div>
          </div>
        </div>

        {/* family name */}
        <div style={{ padding: '14px 20px 0' }}>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 44, lineHeight: 1, color: '#2a1a10', fontWeight: 400, letterSpacing: -0.5 }}>
            {FAMILY_NAME}
          </div>
          <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#7a5a3a', marginTop: 6, letterSpacing: 0.1 }}>
            Eight people · 4,218 moments · since 2019
          </div>
        </div>

        {/* mantlepiece — a wood shelf with family avatars */}
        <div style={{ padding: '22px 0 0', position: 'relative' }}>
          <WoodSurface tone={woodTone} style={{ margin: '0 14px', height: 120, borderRadius: 20, boxShadow: '0 12px 30px -10px rgba(60,30,10,0.4), 0 2px 0 rgba(255,240,210,0.35) inset' }}>
            <div style={{
              position: 'absolute', inset: 0, display: 'flex', alignItems: 'center',
              gap: 16, padding: '0 18px', overflowX: 'auto',
            }}>
              {MEMBERS.map((m, i) => (
                <div key={i} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, flexShrink: 0 }}>
                  <Avatar name={m.n} hue={m.hue} size={52} ring={i===2} />
                  <div style={{ fontFamily: 'system-ui', fontSize: 11, color: '#fff4d6', letterSpacing: 0.3, textShadow: '0 1px 2px rgba(0,0,0,0.5)' }}>{m.n}</div>
                </div>
              ))}
            </div>
            {/* edge shadow under shelf */}
          </WoodSurface>
          <div style={{
            margin: '0 14px', height: 14, borderRadius: 20,
            background: 'linear-gradient(180deg, rgba(40,20,10,0.25) 0%, transparent 100%)',
            marginTop: -4, pointerEvents: 'none',
          }} />
        </div>

        {/* section header */}
        <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', padding: '24px 22px 10px' }}>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 22, color: '#2a1a10' }}>
            Lately in the room
          </div>
          <div style={{ fontFamily: 'system-ui', fontSize: 13, color: '#9a7040' }}>See all</div>
        </div>

        {/* feed */}
        <div style={{ padding: '0 16px 110px', display: 'flex', flexDirection: 'column', gap: 18 }}>
          {moments.map((m, i) => (
            <MomentCard key={i} {...m} large={i===0} />
          ))}
        </div>
      </HeaderPad>

      {/* bottom glass tab bar */}
      <div style={{ position: 'absolute', bottom: 20, left: 20, right: 20, zIndex: 10 }}>
        <div style={{
          height: 64, borderRadius: 32, position: 'relative', overflow: 'hidden',
          backdropFilter: 'blur(20px) saturate(180%)', WebkitBackdropFilter: 'blur(20px) saturate(180%)',
          background: 'rgba(255,245,220,0.55)',
          boxShadow: '0 10px 30px rgba(80,40,10,0.18), inset 0 1px 0 rgba(255,255,255,0.7), 0 0 0 0.5px rgba(120,80,40,0.15)',
          display: 'flex', alignItems: 'center', justifyContent: 'space-around', padding: '0 14px',
        }}>
          {[
            { i: Icon.home, l: 'Room', on: true },
            { i: Icon.calendar, l: 'Memory' },
            { i: Icon.plus, l: 'Add', fab: true },
            { i: Icon.people, l: 'Family' },
            { i: Icon.search, l: 'Search' },
          ].map((t, i) => (
            t.fab ? (
              <div key={i} style={{
                width: 52, height: 52, borderRadius: 26, marginTop: -18,
                background: 'linear-gradient(160deg, #e8a95a, #b36a2a)',
                boxShadow: '0 6px 16px rgba(140,60,10,0.4), inset 0 1px 0 rgba(255,230,180,0.6)',
                display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#fff',
              }}>{t.i(26, '#fff')}</div>
            ) : (
              <div key={i} style={{
                display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2,
                color: t.on ? '#5a2a0a' : '#8a6540',
              }}>
                {t.i(22, t.on ? '#5a2a0a' : '#8a6540')}
                <div style={{ fontFamily: 'system-ui', fontSize: 10, fontWeight: t.on ? 600 : 400 }}>{t.l}</div>
              </div>
            )
          ))}
        </div>
      </div>
    </ScreenShell>
  );
}

function MomentCard({ t, s, hue, by, ago, large }) {
  return (
    <div style={{
      borderRadius: 22, overflow: 'hidden', position: 'relative',
      background: '#fff8ea',
      boxShadow: '0 2px 6px rgba(80,40,10,0.08), 0 14px 30px -12px rgba(80,40,10,0.18), 0 0 0 0.5px rgba(140,90,50,0.12)',
    }}>
      <PhotoPlaceholder label={t.toUpperCase().replace(/[^A-Z0-9 ]/g, '').slice(0,18)} hue={hue} ratio={large ? 4/3 : 16/11} />
      {/* paper margin — thin cream stripe */}
      <div style={{ padding: '14px 16px', display: 'flex', alignItems: 'center', gap: 10 }}>
        <Avatar name={by} hue={MEMBERS.find(m=>m.n===by)?.hue || 30} size={30} />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 19, color: '#2a1a10', lineHeight: 1.15 }}>{t}</div>
          <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#8a6540', marginTop: 2 }}>{s} · {by} · {ago}</div>
        </div>
        <div style={{ color: '#b08860', display: 'flex', gap: 10 }}>{Icon.heart(20, '#b08860')}</div>
      </div>
    </div>
  );
}

window.HomeScreen = HomeScreen;
window.Avatar = Avatar;
window.ScreenShell = ScreenShell;
window.HeaderPad = HeaderPad;
window.MEMBERS = MEMBERS;
window.FAMILY_NAME = FAMILY_NAME;
