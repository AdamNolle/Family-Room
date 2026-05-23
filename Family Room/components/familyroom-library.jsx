// Library scatter view — from the user's sketch.
// "Family Room" + Years row on the left; on the right a wall of slightly-tilted
// polaroids; a thin pill-timeline of colored event dots along the bottom.

function LibraryScatter() {
  const years = [
    { y: 2024, n: '1,418', active: true },
    { y: 2023, n: '2,041' },
    { y: 2022, n: '1,884' },
    { y: 2021, n: '1,332' },
    { y: 2020, n: '987' },
    { y: 2019, n: '1,512' },
  ];

  // 12 photos in a loose 3-col / 4-row grid; per-card rotation gives the scattered feel.
  const scenes = ['lake','sunset','cake','indoor','garden','table','stage','bike','studio','fall','indoor','sunset'];
  const captions = [
    'Aug 9 — the dock',
    'Aug 9 — candles',
    'Aug 10 — morning swim',
    "Iris's bike — Jul 2",
    'Roses, before rain',
    'Sunday brunch',
    'School play, Sep',
    'The toast',
    "Theo's studio",
    'Apple picking',
    'Friday at home',
    'Boat at dusk',
  ];
  const rots = [-3.2, 1.8, -1.4, 2.6, -2.1, 1.2, -3.4, 0.8, 2.2, -1.7, 1.5, -2.8];

  // Bottom timeline — colored event dots across the year (red = celebration,
  // green = travel, blue = everyday, amber = milestone)
  const events = [
    { pct: 4,   tone: '#c8503e', label: "New Year's brunch" },
    { pct: 14,  tone: '#3a6a8c', label: 'A quiet February' },
    { pct: 24,  tone: '#5a8a6a', label: 'Iris turns six' },
    { pct: 36,  tone: '#3a6a8c', label: 'Spring weekends' },
    { pct: 48,  tone: '#d49060', label: "Margot's gallery show" },
    { pct: 58,  tone: '#5a8a6a', label: 'Summer at the lake' },
    { pct: 64,  tone: '#c8503e', label: "Henry's 72nd",  active: true },
    { pct: 74,  tone: '#3a6a8c', label: 'Back to school' },
    { pct: 84,  tone: '#d49060', label: 'Apple picking' },
    { pct: 96,  tone: '#c8503e', label: 'Holidays' },
  ];

  const activeEvent = events.find(e => e.active);

  return (
    <FeatureShell feature="Albums">
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '320px 1fr',
        gap: 28, minHeight: 0 }}>

        {/* ── Left column: Family Room + Years ────────────── */}
        <aside style={{ display: 'flex', flexDirection: 'column', gap: 26, paddingTop: 8 }}>
          <div>
            <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: 1.2,
              textTransform: 'uppercase', color: '#b86348' }}>The Alderhouse</div>
            <h1 style={{ fontFamily: DISPLAY, fontSize: 44, fontWeight: 500,
              color: '#2a221a', letterSpacing: -0.8, margin: '6px 0 0',
              lineHeight: 1.0 }}>Family Room</h1>
            <p style={{ fontSize: 13.5, color: '#4a4035', marginTop: 10,
              lineHeight: 1.55, maxWidth: 280 }}>
              Eleven thousand and change. Pick a year — the wall rearranges. Or scrub
              the timeline below to find a Tuesday in May.
            </p>
          </div>

          <div>
            <div style={{ fontFamily: DISPLAY, fontSize: 22, fontWeight: 500,
              color: '#2a221a', letterSpacing: -0.3, marginBottom: 12 }}>Years</div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {years.map(y => (
                <button key={y.y}
                  aria-pressed={y.active}
                  style={{
                    display: 'flex', alignItems: 'baseline', gap: 12,
                    padding: '14px 18px', borderRadius: 14, border: 'none',
                    background: y.active
                      ? 'linear-gradient(145deg, #f4ecdd, #d8cfba)'
                      : 'linear-gradient(145deg, #f4ecdd, #e0d6c2)',
                    boxShadow: y.active ? NEU_IN_SM : NEU_OUT_SM,
                    cursor: 'pointer',
                    color: y.active ? '#b86348' : '#4a4035',
                    fontFamily: SANS, textAlign: 'left',
                  }}>
                  <span style={{ fontFamily: DISPLAY, fontSize: 22, fontWeight: 500,
                    letterSpacing: -0.4 }}>{y.y}</span>
                  <span style={{ flex: 1 }}/>
                  <span style={{ fontSize: 11.5, color: y.active ? '#b86348' : '#8a7e70',
                    fontVariantNumeric: 'tabular-nums' }}>{y.n}</span>
                </button>
              ))}
            </div>
          </div>

          <div style={{ flex: 1 }}/>

          <div style={{ padding: 14, borderRadius: 14, background: CREAM,
            boxShadow: NEU_IN_SM, fontSize: 11.5, color: '#6e6358', lineHeight: 1.55 }}>
            <div style={{ fontSize: 10.5, fontWeight: 600, color: '#b86348',
              textTransform: 'uppercase', letterSpacing: 0.8, marginBottom: 4 }}>
              Now showing
            </div>
            2024 — <strong style={{ color: '#2a221a' }}>1,418 photos</strong> across
            29 trips, holidays and ordinary Tuesdays.
          </div>
        </aside>

        {/* ── Right column: Library wall ──────────────────── */}
        <section style={{ display: 'flex', flexDirection: 'column', minHeight: 0, gap: 16 }}>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between' }}>
            <div style={{ fontFamily: DISPLAY, fontSize: 32, fontWeight: 500,
              color: '#2a221a', letterSpacing: -0.5 }}>Library</div>
            <div style={{ display: 'flex', gap: 8 }}>
              <NeuChip active>Everyone</NeuChip>
              <NeuChip>Just Iris</NeuChip>
              <NeuChip>Just me</NeuChip>
              <NeuChip icon="film">Hide videos</NeuChip>
            </div>
          </div>

          {/* The wall — neumorphic well, polaroids scattered inside */}
          <div style={{ flex: 1, padding: 26, borderRadius: 22,
            background: 'linear-gradient(145deg, #e6dcc6, #f1e8d4)',
            boxShadow: NEU_IN, position: 'relative', overflow: 'hidden' }}>

            {/* subtle paper texture via radial gradients */}
            <div aria-hidden="true" style={{ position: 'absolute', inset: 0,
              background: 'radial-gradient(circle at 20% 30%, rgba(255,250,238,0.4) 0%, transparent 50%), radial-gradient(circle at 80% 70%, rgba(120,95,60,0.06) 0%, transparent 60%)',
              pointerEvents: 'none' }}/>

            <div style={{ position: 'relative', display: 'grid',
              gridTemplateColumns: 'repeat(3, 1fr)',
              gridTemplateRows: 'repeat(4, 1fr)',
              gap: 14, height: '100%' }}>
              {scenes.map((s, i) => {
                const r = rots[i];
                const lifted = i === 4; // the one being focused
                return (
                  <article key={i}
                    aria-label={captions[i]}
                    style={{
                      position: 'relative',
                      transform: `rotate(${r}deg) ${lifted ? 'translateY(-6px) scale(1.04)' : ''}`,
                      transition: 'transform 220ms ease',
                      filter: lifted ? 'none' : 'none',
                      zIndex: lifted ? 5 : 1,
                    }}>
                    {/* polaroid frame */}
                    <div style={{
                      background: '#fbf6ea', padding: '8px 8px 26px',
                      borderRadius: 4,
                      boxShadow: lifted
                        ? '0 14px 28px rgba(40,30,20,0.28), 0 2px 4px rgba(40,30,20,0.16)'
                        : '0 4px 10px rgba(40,30,20,0.18), 0 1px 2px rgba(40,30,20,0.12)',
                      height: '100%', display: 'flex', flexDirection: 'column',
                    }}>
                      <div style={{ flex: 1, position: 'relative', overflow: 'hidden',
                        borderRadius: 1, background: '#000' }}>
                        <Photo scene={s} round={0} style={{ position: 'absolute', inset: 0 }}/>
                      </div>
                      <div style={{ marginTop: 6, fontFamily: '"Caveat", "Bradley Hand", cursive',
                        fontSize: 13, color: '#4a4035', textAlign: 'center',
                        whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                        {captions[i]}
                      </div>
                    </div>
                  </article>
                );
              })}
            </div>
          </div>

          {/* Pill timeline */}
          <div aria-label="Year timeline" style={{ position: 'relative',
            height: 56, borderRadius: 28,
            background: 'linear-gradient(145deg, #eef4f8, #dde8f0)',
            boxShadow: NEU_IN_SM,
            display: 'flex', alignItems: 'center', padding: '0 22px' }}>

            {/* month ticks */}
            <div aria-hidden="true" style={{ position: 'absolute', left: 22, right: 22,
              top: '50%', transform: 'translateY(-50%)', height: 1,
              background: 'rgba(58,106,140,0.18)' }}/>
            <div aria-hidden="true" style={{ position: 'absolute', left: 22, right: 22,
              top: 0, bottom: 0, display: 'flex', justifyContent: 'space-between',
              alignItems: 'center', pointerEvents: 'none' }}>
              {['Jan','Mar','May','Jul','Sep','Nov'].map(m => (
                <div key={m} style={{ fontSize: 10, fontWeight: 600,
                  color: '#7a8c9a', letterSpacing: 0.6,
                  textTransform: 'uppercase' }}>{m}</div>
              ))}
            </div>

            {/* event dots */}
            <div style={{ position: 'absolute', left: 22, right: 22, top: 0, bottom: 0 }}>
              {events.map((e, i) => (
                <button key={i}
                  aria-label={e.label}
                  style={{
                    position: 'absolute', top: '50%',
                    left: `${e.pct}%`,
                    transform: 'translate(-50%, -50%)',
                    width: e.active ? 22 : 16, height: e.active ? 22 : 16,
                    borderRadius: '50%', border: 'none',
                    background: `radial-gradient(circle at 35% 30%, ${e.tone}f0 0%, ${e.tone} 65%, ${e.tone}aa 100%)`,
                    boxShadow: e.active
                      ? `0 0 0 4px rgba(255,250,238,0.95), 0 4px 10px ${e.tone}80, inset 0 1px 1px rgba(255,255,255,0.5)`
                      : `0 2px 4px rgba(40,30,20,0.25), inset 0 1px 1px rgba(255,255,255,0.4)`,
                    cursor: 'pointer',
                    zIndex: e.active ? 3 : 2,
                  }}/>
              ))}

              {/* tooltip on the active dot */}
              {activeEvent && (
                <div style={{ position: 'absolute',
                  top: '-28px', left: `${activeEvent.pct}%`,
                  transform: 'translateX(-50%)',
                  padding: '5px 10px', borderRadius: 8,
                  background: '#2a221a', color: '#f6efe0',
                  fontSize: 11, fontWeight: 600,
                  whiteSpace: 'nowrap', pointerEvents: 'none',
                  boxShadow: '0 4px 10px rgba(40,30,20,0.3)' }}>
                  {activeEvent.label}
                  <div style={{ position: 'absolute', bottom: -4, left: '50%',
                    transform: 'translateX(-50%) rotate(45deg)',
                    width: 8, height: 8, background: '#2a221a' }}/>
                </div>
              )}
            </div>
          </div>
        </section>
      </div>
    </FeatureShell>
  );
}

window.LibraryScatter = LibraryScatter;
