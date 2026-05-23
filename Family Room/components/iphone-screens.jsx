// iPhone screens v3 — soft springy fabric, cozy glass, SF Rounded
// Aligned with the repo's "Cozy Glass" framework: rounded type, warm tints, organic shadows.

const MEMBERS = [
  { n: 'Margot', r: 'Grandma', tone: 'rose',     count: 487 },
  { n: 'Henry',  r: 'Grandpa', tone: 'mustard',  count: 412 },
  { n: 'Clara',  r: 'Mom',     tone: 'claret',   count: 891 },
  { n: 'Theo',   r: 'Dad',     tone: 'moss',     count: 654 },
  { n: 'Iris',   r: 'Daughter',tone: 'clay',     count: 1022 },
  { n: 'Jun',    r: 'Son',     tone: 'pine',     count: 344 },
  { n: 'Lena',   r: 'Aunt',    tone: 'lavender', count: 228 },
  { n: 'Marcus', r: 'Uncle',   tone: 'terra',    count: 180 },
];

const YEARS = [
  { y: 2026, c: 218, events: 4 },
  { y: 2025, c: 942, events: 28 },
  { y: 2024, c: 811, events: 24 },
  { y: 2023, c: 698, events: 19 },
  { y: 2022, c: 542, events: 17 },
  { y: 2021, c: 401, events: 13 },
  { y: 2020, c: 378, events: 11 },
  { y: 2019, c: 228, events: 9 },
];

const EVENTS = [
  { t: "Henry's 72nd",         sub: 'Lake house · Aug 2025',    tone: 'mustard', count: 61,  people: 7 },
  { t: "Iris learns to ride",  sub: 'Back road · Jun 2025',     tone: 'sky',     count: 14,  people: 2 },
  { t: "Sunday lunch",         sub: "Margot's kitchen · weekly",tone: 'butter',  count: 124, people: 6 },
  { t: "Spring planting",      sub: 'Garden · Apr 2025',        tone: 'moss',    count: 38,  people: 3 },
  { t: "Jun's first tooth",    sub: 'Home · Mar 2025',          tone: 'rose',    count: 8,   people: 4 },
  { t: "Cabin weekend",        sub: 'Door County · Oct 2024',   tone: 'terra',   count: 89,  people: 5 },
];

const R_FONT = '"SF Pro Rounded", ui-rounded, system-ui, sans-serif';
const S_FONT = '"SF Pro", -apple-system, system-ui, sans-serif';

function AvatarR({ name, tone = 'butter', size = 40, mode, ring }) {
  const initial = name?.[0] || '?';
  const tones = {
    butter:  'radial-gradient(circle at 30% 25%, #fbe9b8, #d7a855)',
    clay:    'radial-gradient(circle at 30% 25%, #f3bca6, #b25238)',
    rose:    'radial-gradient(circle at 30% 25%, #eec2bf, #a66868)',
    claret:  'radial-gradient(circle at 30% 25%, #b56464, #5a1a1a)',
    mustard: 'radial-gradient(circle at 30% 25%, #ecc378, #92610f)',
    moss:    'radial-gradient(circle at 30% 25%, #b0c084, #55683a)',
    pine:    'radial-gradient(circle at 30% 25%, #6e8878, #28403a)',
    lavender:'radial-gradient(circle at 30% 25%, #d5c3da, #816a91)',
    terra:   'radial-gradient(circle at 30% 25%, #cc7a5e, #6e2410)',
    sky:     'radial-gradient(circle at 30% 25%, #b8cedd, #627e98)',
  };
  const bg = tones[tone] || tones.butter;
  const darkInk = ['rose','lavender','butter','sky','clay'].includes(tone);
  return (
    <div style={{
      width: size, height: size, borderRadius: '50%', position: 'relative',
      background: bg,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      color: darkInk ? '#3a1810' : '#f6e6c8',
      fontFamily: R_FONT, fontWeight: 600, fontSize: size * 0.42,
      boxShadow: ring
        ? `0 0 0 2.5px #f4dfb2, 0 0 0 4px rgba(200,100,80,0.5), inset 0 -2px 6px rgba(40,15,10,0.22)`
        : 'inset 0 1.5px 0 rgba(255,245,220,0.45), inset 0 -2px 6px rgba(40,15,10,0.25)',
      flexShrink: 0,
    }}>{initial}</div>
  );
}

function Shell({ children, mode = 'natural', intensity = 1, bg }) {
  // Brighter, creamier base — the light feels like morning through sheer curtains, not a dim amber room.
  const base = mode === 'daylight' ? '#f5ddb4' : (bg || '#faeed3');
  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', background: base, overflow: 'hidden' }}>
      <WindowLight mode={mode} intensity={intensity} />
      <div style={{ position: 'relative', zIndex: 2, height: '100%', overflow: 'auto' }}>
        {children}
      </div>
    </div>
  );
}

const INK = { ink: '#3a1810', soft: '#6a3822', mute: '#a07050' };

// ═══════════════════════════════════════════════════════════════
// iPhone HOME — "The Hearth"
// ═══════════════════════════════════════════════════════════════
function HomeV2({ mode = 'natural', intensity = 1, woodTone = 'walnut' }) {
  return (
    <Shell mode={mode} intensity={intensity}>
      <div style={{ paddingTop: 58 }}>
        {/* tiny meta */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 20px 0' }}>
          <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 600, letterSpacing: 1.8, color: INK.mute, textTransform: 'uppercase' }}>
            Family Room
          </div>
          <CozyGlass radius={17} style={{ width: 34, height: 34, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            {Icon.search(16, INK.soft)}
          </CozyGlass>
        </div>

        {/* title — SF Rounded */}
        <div style={{ padding: '16px 20px 0' }}>
          <div style={{ fontFamily: R_FONT, fontSize: 40, lineHeight: 1.02, color: INK.ink, fontWeight: 700, letterSpacing: -0.8 }}>
            The Alderhouse
          </div>
          <div style={{ fontFamily: R_FONT, fontSize: 13.5, color: INK.soft, marginTop: 6, fontWeight: 500 }}>
            8 in the room · 4,218 memories · since 2019
          </div>
        </div>

        {/* family row — soft pillow avatars on a fabric settee */}
        <div style={{ padding: '18px 14px 0' }}>
          <SoftFabric tone="claret" radius={26} style={{ padding: '14px 12px 16px' }}>
            <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 700, color: 'rgba(246,224,200,0.75)', letterSpacing: 1.4, textTransform: 'uppercase', padding: '0 6px 10px' }}>
              In the room today
            </div>
            <div style={{ display: 'flex', gap: 12, overflowX: 'auto', padding: '0 6px' }}>
              {MEMBERS.map((m, i) => (
                <div key={i} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, flexShrink: 0 }}>
                  <AvatarR name={m.n} tone={m.tone} size={50} ring={i === 2} />
                  <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 600, color: '#f4dfb2' }}>{m.n}</div>
                </div>
              ))}
            </div>
          </SoftFabric>
        </div>

        {/* organize chips — soft pill buttons */}
        <div style={{ padding: '22px 20px 0', display: 'flex', gap: 7, overflowX: 'auto' }}>
          {[
            { l: 'Years',  tone: 'butter', on: true },
            { l: 'People', tone: 'ghost' },
            { l: 'Events', tone: 'ghost' },
            { l: 'Places', tone: 'ghost' },
            { l: 'Albums', tone: 'ghost' },
          ].map((c, i) => (
            <SoftButton key={i} tone={c.on ? 'butter' : 'ghost'}>{c.l}</SoftButton>
          ))}
        </div>

        {/* year shelves */}
        <div style={{ padding: '22px 18px 130px' }}>
          {YEARS.slice(0, 3).map((yr, yi) => (
            <YearShelf key={yi} year={yr} mode={mode} first={yi === 0} />
          ))}
        </div>
      </div>

      {/* cozy glass tab bar */}
      <div style={{ position: 'absolute', bottom: 22, left: 20, right: 20, zIndex: 10 }}>
        <CozyGlass tint="butter" radius={32} style={{
          height: 64, display: 'flex', alignItems: 'center', justifyContent: 'space-around', padding: '0 8px',
        }}>
          {[
            { i: Icon.home, l: 'Room', on: true },
            { i: Icon.calendar, l: 'Memory' },
            { i: Icon.plus, l: 'Add', fab: true },
            { i: Icon.people, l: 'Family' },
            { i: Icon.search, l: 'Find' },
          ].map((t, i) => (
            t.fab ? (
              <div key={i} style={{
                width: 54, height: 54, borderRadius: 27, marginTop: -18,
                background: 'radial-gradient(circle at 30% 25%, #f0a890 0%, #b0492e 100%)',
                boxShadow: [
                  'inset 0 1.5px 0 rgba(255,235,205,0.55)',
                  'inset 0 -2px 4px rgba(60,15,5,0.3)',
                  '0 8px 18px -4px rgba(140,60,35,0.55)',
                ].join(', '),
                display: 'flex', alignItems: 'center', justifyContent: 'center',
              }}>{t.i(24, '#fff4d6')}</div>
            ) : (
              <div key={i} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2, color: t.on ? INK.ink : INK.mute }}>
                {t.i(21, t.on ? INK.ink : INK.mute)}
                <div style={{ fontFamily: R_FONT, fontSize: 10, fontWeight: t.on ? 700 : 500 }}>{t.l}</div>
              </div>
            )
          ))}
        </CozyGlass>
      </div>
    </Shell>
  );
}

function YearShelf({ year, mode, first }) {
  return (
    <div style={{ marginBottom: 28 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', padding: '0 6px 12px' }}>
        <div>
          <div style={{ fontFamily: R_FONT, fontSize: 28, color: INK.ink, fontWeight: 700, lineHeight: 1, letterSpacing: -0.4 }}>{year.y}</div>
          <div style={{ fontFamily: R_FONT, fontSize: 11.5, fontWeight: 500, color: INK.soft, marginTop: 4 }}>{year.c} memories · {year.events} events</div>
        </div>
        <div style={{ fontFamily: R_FONT, fontSize: 12, fontWeight: 600, color: INK.soft }}>See all →</div>
      </div>
      <div style={{ display: 'flex', gap: 9, overflowX: 'auto', paddingBottom: 4 }}>
        {EVENTS.slice(first ? 0 : 2, first ? 4 : 6).map((e, i) => (
          <SoftFabric key={i} tone={e.tone} radius={18} style={{ width: 134, flexShrink: 0, padding: 6 }} pressable>
            <div style={{ borderRadius: 13, overflow: 'hidden', boxShadow: 'inset 0 0 0 0.5px rgba(40,15,10,0.14)' }}>
              <Photo tone={e.tone} label={e.t.slice(0, 14)} ratio={4/5} mode={mode} />
            </div>
            <div style={{ padding: '8px 4px 4px' }}>
              <div style={{ fontFamily: R_FONT, fontSize: 13, fontWeight: 700, lineHeight: 1.15 }}>{e.t}</div>
              <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 500, opacity: 0.65, marginTop: 3 }}>{e.count} · {e.people} people</div>
            </div>
          </SoftFabric>
        ))}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// iPhone PEOPLE
// ═══════════════════════════════════════════════════════════════
function OrganizeV2({ mode = 'natural', intensity = 1 }) {
  return (
    <Shell mode={mode} intensity={intensity}>
      <div style={{ paddingTop: 58 }}>
        <div style={{ padding: '0 20px' }}>
          <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.8, color: INK.mute, textTransform: 'uppercase' }}>Browse</div>
          <div style={{ fontFamily: R_FONT, fontSize: 40, color: INK.ink, fontWeight: 700, marginTop: 4, lineHeight: 1, letterSpacing: -0.8 }}>People</div>
        </div>

        <div style={{ padding: '14px 20px 0', display: 'flex', gap: 7, overflowX: 'auto' }}>
          {['Years','People','Events','Places','Albums'].map((l, i) => (
            <SoftButton key={i} tone={l === 'People' ? 'butter' : 'ghost'}>{l}</SoftButton>
          ))}
        </div>

        <div style={{ padding: '20px 20px 26px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
          {MEMBERS.slice(0, 4).map((m, i) => (
            <SoftFabric key={i} tone={m.tone} radius={20} style={{ padding: 7 }} pressable>
              <div style={{ borderRadius: 14, overflow: 'hidden' }}>
                <Photo tone={m.tone} label={m.n} ratio={1} mode={mode} />
              </div>
              <div style={{ padding: '10px 6px 4px', display: 'flex', alignItems: 'center', gap: 9 }}>
                <AvatarR name={m.n} tone={m.tone} size={26} />
                <div>
                  <div style={{ fontFamily: R_FONT, fontSize: 14, fontWeight: 700, lineHeight: 1 }}>{m.n}</div>
                  <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 500, opacity: 0.65, marginTop: 2 }}>{m.count} memories</div>
                </div>
              </div>
            </SoftFabric>
          ))}
        </div>

        <div style={{ padding: '0 20px 10px' }}>
          <div style={{ fontFamily: R_FONT, fontSize: 22, color: INK.ink, fontWeight: 700 }}>Places</div>
        </div>
        <div style={{ padding: '0 20px 130px', display: 'flex', flexDirection: 'column', gap: 10 }}>
          {[
            { p: "Margot's kitchen",  n: 312, tone: 'butter' },
            { p: 'Lake house',        n: 218, tone: 'sky' },
            { p: 'Back road',         n: 88,  tone: 'moss' },
            { p: 'Door County cabin', n: 142, tone: 'terra' },
          ].map((pl, i) => (
            <SoftFabric key={i} tone="butter" radius={18} style={{ padding: '10px 12px', display: 'flex', alignItems: 'center', gap: 12 }} pressable>
              <div style={{ width: 44, height: 44, borderRadius: 11, overflow: 'hidden', flexShrink: 0, boxShadow: 'inset 0 0 0 0.5px rgba(40,15,10,0.12)' }}>
                <Photo tone={pl.tone} ratio={1} mode={mode} />
              </div>
              <div style={{ flex: 1 }}>
                <div style={{ fontFamily: R_FONT, fontSize: 15, fontWeight: 700, color: INK.ink }}>{pl.p}</div>
                <div style={{ fontFamily: R_FONT, fontSize: 11.5, fontWeight: 500, color: INK.soft, marginTop: 1 }}>{pl.n} memories</div>
              </div>
              {Icon.chevronR(16, INK.mute)}
            </SoftFabric>
          ))}
        </div>
      </div>
    </Shell>
  );
}

// ═══════════════════════════════════════════════════════════════
// iPhone MEMORY — polaroid on fabric
// ═══════════════════════════════════════════════════════════════
function MemoryV2({ mode = 'natural', intensity = 1 }) {
  return (
    <Shell mode={mode} intensity={intensity}>
      <div style={{ paddingTop: 58 }}>
        <div style={{ padding: '0 20px' }}>
          <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.8, color: INK.mute, textTransform: 'uppercase' }}>On this day</div>
          <div style={{ fontFamily: R_FONT, fontSize: 34, color: INK.ink, fontWeight: 700, lineHeight: 1.05, marginTop: 6, letterSpacing: -0.6 }}>
            Three Aprils ago
          </div>
          <div style={{ fontFamily: R_FONT, fontSize: 13.5, fontWeight: 500, color: INK.soft, marginTop: 8, lineHeight: 1.5 }}>
            Henry came home from the hospital. 18 photos, 2 videos, and a quiet spring afternoon.
          </div>
        </div>

        <div style={{ padding: '22px 22px 0' }}>
          <SoftFabric tone="claret" radius={20} style={{ padding: 14 }}>
            <SoftFabric tone="butter" radius={8} raised={false} style={{ padding: 10 }}>
              <div style={{ borderRadius: 3, overflow: 'hidden', boxShadow: 'inset 0 0 0 0.5px rgba(40,15,10,0.14)' }}>
                <Photo tone="butter" label="APRIL 20 · 2023 · HOME" ratio={4/5} mode={mode} />
              </div>
              <div style={{ fontFamily: R_FONT, fontStyle: 'italic', fontSize: 14, fontWeight: 500, color: INK.soft, textAlign: 'center', padding: '12px 0 2px' }}>
                "Welcome home, Dad."
              </div>
            </SoftFabric>
          </SoftFabric>
        </div>

        <div style={{ padding: '22px 0 0' }}>
          <div style={{ padding: '0 22px', fontFamily: R_FONT, fontSize: 18, fontWeight: 700, color: INK.ink, marginBottom: 10 }}>
            The rest of that day
          </div>
          <div style={{ display: 'flex', gap: 9, overflowX: 'auto', padding: '0 20px 10px' }}>
            {['butter','clay','mustard','sky','rose','moss','terra'].map((tn, i) => (
              <SoftFabric key={i} tone={tn} radius={14} style={{ width: 108, flexShrink: 0, padding: 5 }} pressable>
                <div style={{ borderRadius: 10, overflow: 'hidden' }}>
                  <Photo tone={tn} ratio={3/4} mode={mode} />
                </div>
              </SoftFabric>
            ))}
          </div>
        </div>

        <div style={{ padding: '22px 20px 130px' }}>
          <CozyGlass tint="butter" radius={20} style={{ padding: '14px 14px', display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{
              width: 46, height: 46, borderRadius: 23,
              background: 'radial-gradient(circle at 30% 25%, #f0a890, #b0492e)',
              boxShadow: 'inset 0 1.5px 0 rgba(255,235,205,0.55), 0 6px 14px -4px rgba(140,60,35,0.5)',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
            }}>{Icon.play(20, '#fff4d6')}</div>
            <div style={{ flex: 1 }}>
              <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 700, color: INK.mute, letterSpacing: 1.2, textTransform: 'uppercase' }}>Memory reel</div>
              <div style={{ fontFamily: R_FONT, fontSize: 16, fontWeight: 700, color: INK.ink, marginTop: 2 }}>A quiet spring afternoon</div>
              <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 500, color: INK.soft, marginTop: 1 }}>2 min · curated for Clara</div>
            </div>
          </CozyGlass>
        </div>
      </div>
    </Shell>
  );
}

Object.assign(window, { HomeV2, OrganizeV2, MemoryV2, AvatarR, Shell, INK, MEMBERS, YEARS, EVENTS, R_FONT });
