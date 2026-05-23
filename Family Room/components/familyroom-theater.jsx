// Memories theater — browse and watch family movies / autoplay reels.

function MemoryTheater() {
  const reels = [
    { id: 1, title: 'Henry, seventy-two', sub: "Aug 9 · 4:12", scene: 'cake', who: 'Everyone', auto: false, hero: true },
    { id: 2, title: 'A summer at the lake', sub: 'Jun 02 — Sep 04 · 6:48', scene: 'lake', who: 'Everyone', auto: true },
    { id: 3, title: 'Iris on stage', sub: 'Sep 22 · 2:30', scene: 'stage', who: 'Iris, Cora', auto: false },
    { id: 4, title: 'A year together', sub: '2024 · 12:04', scene: 'sunset', who: 'Everyone', auto: true },
    { id: 5, title: 'Theo in the studio', sub: 'May–Jul · 5:18', scene: 'studio', who: 'Theo', auto: true },
    { id: 6, title: 'Apple picking', sub: 'Oct 11 · 1:42', scene: 'fall', who: 'Margot, Cora, Iris', auto: false },
  ];
  const upNext = reels.slice(2);
  const featured = reels[0];

  return (
    <FeatureShell feature="Memories">
      <SectionHead
        kicker="Memories"
        title="The family theater"
        sub="Hand-edited films and the gentle auto-reels we make for you each week. Sit down with a coffee or cast it to the TV.">
        <NeuButton icon="film">Cast to Apple TV</NeuButton>
        <NeuButton icon="plus" primary>New memory</NeuButton>
      </SectionHead>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1.7fr 1fr', gap: 16, minHeight: 0 }}>

        {/* ── Hero player ───────────────────────────────── */}
        <section aria-label="Now playing" style={{ display: 'flex', flexDirection: 'column', minHeight: 0, gap: 14 }}>
          <div style={{ position: 'relative', borderRadius: 20, overflow: 'hidden',
            background: '#000', boxShadow: NEU_OUT, aspectRatio: '16 / 9' }}>
            <Photo scene={featured.scene} round={0}/>
            <div style={{ position: 'absolute', inset: 0,
              background: 'linear-gradient(180deg, transparent 30%, rgba(0,0,0,0.7) 100%)' }}/>

            {/* Center play button */}
            <button aria-label="Play movie" style={{ position: 'absolute', top: '50%', left: '50%',
              transform: 'translate(-50%, -50%)',
              width: 84, height: 84, borderRadius: 42, border: 'none',
              background: 'rgba(255,250,238,0.92)',
              boxShadow: '0 8px 30px rgba(0,0,0,0.4)',
              cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <Icon name="play" size={32} color="#b86348"/>
            </button>

            {/* Lower-third copy */}
            <div style={{ position: 'absolute', left: 28, right: 28, bottom: 22, color: '#fff' }}>
              <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: 1.4,
                textTransform: 'uppercase', opacity: 0.85 }}>Featured · Edited by Cora</div>
              <div style={{ fontFamily: DISPLAY, fontSize: 38, fontWeight: 500, marginTop: 6,
                letterSpacing: -0.6, lineHeight: 1.05 }}>{featured.title}</div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 14, marginTop: 8, fontSize: 13, opacity: 0.9 }}>
                <span>{featured.sub}</span>
                <span style={{ width: 3, height: 3, borderRadius: 2, background: 'currentColor', opacity: 0.5 }}/>
                <span>The Lake House</span>
                <span style={{ width: 3, height: 3, borderRadius: 2, background: 'currentColor', opacity: 0.5 }}/>
                <span>Everyone</span>
              </div>
              <div style={{ display: 'flex', marginTop: 12 }}>
                {PEOPLE.map((p,i) => (
                  <div key={p.id} style={{ marginLeft: i === 0 ? 0 : -10 }}>
                    <Avatar p={p} size={30} ring/>
                  </div>
                ))}
              </div>
            </div>

            {/* Top-right meta */}
            <div style={{ position: 'absolute', top: 14, right: 14, display: 'flex', gap: 6 }}>
              <div style={{ padding: '4px 10px', borderRadius: 999, background: 'rgba(255,255,255,0.18)',
                backdropFilter: 'blur(8px)', color: '#fff', fontSize: 11, fontWeight: 600 }}>
                4K · Stereo
              </div>
            </div>
          </div>

          {/* Below-player rail */}
          <NeuCard padding={14}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
              <Icon name="sparkles" size={14} color="#b86348"/>
              <div style={{ fontSize: 11.5, fontWeight: 600, color: '#b86348',
                textTransform: 'uppercase', letterSpacing: 0.8 }}>This week's auto-reel</div>
              <div style={{ flex: 1 }}/>
              <div style={{ fontSize: 11, color: '#6e6358' }}>made Sunday morning</div>
            </div>
            <div style={{ display: 'flex', gap: 12 }}>
              <div style={{ position: 'relative', borderRadius: 12, overflow: 'hidden',
                width: 160, height: 100, flexShrink: 0 }}>
                <Photo scene="garden" round={0}/>
                <div style={{ position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.25)' }}/>
                <div style={{ position: 'absolute', top: '50%', left: '50%',
                  transform: 'translate(-50%,-50%)', width: 36, height: 36, borderRadius: 18,
                  background: 'rgba(255,250,238,0.9)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <Icon name="play" size={14} color="#b86348"/>
                </div>
                <div style={{ position: 'absolute', bottom: 6, right: 8, padding: '2px 6px',
                  borderRadius: 4, background: 'rgba(0,0,0,0.6)', color: '#fff',
                  fontSize: 10, fontFamily: 'JetBrains Mono, monospace' }}>1:24</div>
              </div>
              <div style={{ flex: 1 }}>
                <div style={{ fontFamily: DISPLAY, fontSize: 18, fontWeight: 500, color: '#2a221a',
                  letterSpacing: -0.3 }}>The week, in a minute</div>
                <div style={{ fontSize: 12.5, color: '#4a4035', marginTop: 6, lineHeight: 1.5 }}>
                  18 photos and 4 short videos from this week — Iris's bike ride, the roses, Theo's new sketch.
                  We'll keep it for two weeks unless you save it.
                </div>
                <div style={{ display: 'flex', gap: 6, marginTop: 10 }}>
                  <NeuButton size="sm" icon="check">Save</NeuButton>
                  <NeuButton size="sm">Skip this week</NeuButton>
                </div>
              </div>
            </div>
          </NeuCard>
        </section>

        {/* ── Up next list ──────────────────────────────── */}
        <aside aria-label="More memories" style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
            <Icon name="film" size={14} color="#4a4035"/>
            <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>Up next · {upNext.length}</div>
            <div style={{ flex: 1 }}/>
            <NeuChip active>All</NeuChip>
            <NeuChip>Saved</NeuChip>
            <NeuChip>Auto-reels</NeuChip>
          </div>

          <div style={{ flex: 1, padding: 12, borderRadius: 16, background: CREAM,
            boxShadow: NEU_IN_SM, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 10 }}>
            {upNext.map(r => (
              <article key={r.id} style={{ display: 'flex', gap: 12, padding: 10, borderRadius: 14,
                background: CREAM, boxShadow: NEU_OUT_SM }}>
                <div style={{ position: 'relative', borderRadius: 10, overflow: 'hidden',
                  width: 110, height: 70, flexShrink: 0 }}>
                  <Photo scene={r.scene} round={0}/>
                  <div style={{ position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.2)' }}/>
                  <div style={{ position: 'absolute', top: '50%', left: '50%',
                    transform: 'translate(-50%,-50%)', width: 28, height: 28, borderRadius: 14,
                    background: 'rgba(255,250,238,0.92)', display: 'flex',
                    alignItems: 'center', justifyContent: 'center' }}>
                    <Icon name="play" size={12} color="#b86348"/>
                  </div>
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    {r.auto && (
                      <div style={{ padding: '1px 6px', borderRadius: 4, background: CREAM,
                        boxShadow: NEU_IN_SM, fontSize: 9.5, fontWeight: 600, color: '#b86348',
                        textTransform: 'uppercase', letterSpacing: 0.6 }}>auto</div>
                    )}
                    <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a',
                      whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{r.title}</div>
                  </div>
                  <div style={{ fontSize: 11, color: '#6e6358', marginTop: 2 }}>{r.sub}</div>
                  <div style={{ fontSize: 11, color: '#8a7e70', marginTop: 4 }}>with {r.who}</div>
                </div>
              </article>
            ))}
          </div>
        </aside>
      </div>
    </FeatureShell>
  );
}

window.MemoryTheater = MemoryTheater;
