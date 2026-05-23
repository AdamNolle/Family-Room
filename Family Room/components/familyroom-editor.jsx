// Memory editor — turn a moment into a short film.
// Three rows: preview + inspector, then media bin, then a multi-track timeline.

function MemoryEditor() {
  const clips = [
    { id: 1, scene: 'sunset', dur: 3.4, label: 'Drive in' },
    { id: 2, scene: 'lake',   dur: 5.2, label: 'On the dock' },
    { id: 3, scene: 'cake',   dur: 4.8, label: 'Candles', selected: true },
    { id: 4, scene: 'indoor', dur: 6.0, label: 'The toast' },
    { id: 5, scene: 'sunset', dur: 4.2, label: 'Boat at dusk' },
  ];
  const totalDur = clips.reduce((a,c) => a + c.dur, 0);
  const selected = clips.find(c => c.selected);
  const playhead = clips.slice(0, clips.findIndex(c => c.selected) + 1).reduce((a,c) => a + c.dur, 0) - selected.dur / 2;

  return (
    <FeatureShell feature="Editor" height={880}>
      <SectionHead
        kicker="Memory editor"
        title="Dad's 72nd — a four-minute version"
        sub="Pick clips, drop them on the timeline, the rhythm follows. Music and titles are optional. Save as a Memory anyone in the family can watch.">
        <NeuButton icon="share">Share preview</NeuButton>
        <NeuButton icon="check" primary>Save memory</NeuButton>
      </SectionHead>

      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 14, minHeight: 0 }}>

        {/* Top row — preview + inspector */}
        <div style={{ flex: '0 0 auto', display: 'grid', gridTemplateColumns: '1.6fr 1fr', gap: 14, height: 320 }}>
          {/* Preview */}
          <div style={{ position: 'relative', borderRadius: 18, overflow: 'hidden',
            background: '#000', boxShadow: NEU_IN_SM }}>
            <div style={{ position: 'absolute', inset: 0 }}>
              <Photo scene={selected.scene} round={0}/>
            </div>
            <div style={{ position: 'absolute', inset: 0,
              background: 'linear-gradient(180deg, transparent 60%, rgba(0,0,0,0.55) 100%)' }}/>
            {/* Title overlay being edited */}
            <div style={{ position: 'absolute', left: 24, bottom: 60, color: '#fff',
              fontFamily: DISPLAY, fontSize: 28, fontWeight: 500, letterSpacing: -0.4 }}>
              Henry, seventy-two
              <div style={{ fontSize: 13, opacity: 0.85, fontFamily: SANS, fontWeight: 500, marginTop: 4 }}>
                The Lake House · August 9
              </div>
            </div>

            {/* Transport */}
            <div style={{ position: 'absolute', left: 16, right: 16, bottom: 14,
              display: 'flex', alignItems: 'center', gap: 12, color: '#fff' }}>
              <button aria-label="Play" style={{ width: 38, height: 38, borderRadius: 19,
                border: 'none', background: 'rgba(255,255,255,0.15)', backdropFilter: 'blur(8px)',
                cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <Icon name="play" size={18} color="#fff"/>
              </button>
              <div style={{ fontFamily: 'JetBrains Mono, ui-monospace, monospace', fontSize: 12, opacity: 0.9 }}>
                01:08 / {Math.floor(totalDur/60).toString().padStart(2,'0')}:{Math.floor(totalDur%60).toString().padStart(2,'0')}
              </div>
              <div style={{ flex: 1, height: 4, borderRadius: 2, background: 'rgba(255,255,255,0.2)',
                position: 'relative' }}>
                <div style={{ position: 'absolute', left: 0, top: 0, bottom: 0, width: '34%',
                  borderRadius: 2, background: '#f0bfa8' }}/>
                <div style={{ position: 'absolute', left: '34%', top: -3, width: 10, height: 10,
                  borderRadius: 5, background: '#fff', boxShadow: '0 1px 3px rgba(0,0,0,0.4)' }}/>
              </div>
              <button aria-label="Volume" style={{ width: 32, height: 32, borderRadius: 10,
                border: 'none', background: 'rgba(255,255,255,0.15)', cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <Icon name="volume" size={16} color="#fff"/>
              </button>
              <button aria-label="Fullscreen" style={{ width: 32, height: 32, borderRadius: 10,
                border: 'none', background: 'rgba(255,255,255,0.15)', cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <Icon name="fullscreen" size={16} color="#fff"/>
              </button>
            </div>
          </div>

          {/* Inspector — what's selected */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10, minHeight: 0 }}>
            <NeuCard padding={14}>
              <div style={{ fontSize: 11, fontWeight: 600, color: '#6e6358',
                textTransform: 'uppercase', letterSpacing: 0.8 }}>Selected clip</div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginTop: 8 }}>
                <Photo scene={selected.scene} w={56} h={56} round={10}/>
                <div style={{ flex: 1 }}>
                  <div style={{ fontFamily: DISPLAY, fontSize: 17, fontWeight: 500, color: '#2a221a' }}>{selected.label}</div>
                  <div style={{ fontSize: 11, color: '#6e6358', marginTop: 2 }}>4.8s · IMG_2841 · Aug 9 6:14 pm</div>
                </div>
              </div>
              <div style={{ display: 'flex', gap: 6, marginTop: 12 }}>
                <NeuChip active>Trim</NeuChip>
                <NeuChip>Speed</NeuChip>
                <NeuChip>Color</NeuChip>
              </div>
              <div style={{ marginTop: 12 }}>
                <label style={{ fontSize: 11, color: '#6e6358', fontWeight: 600 }}>Trim</label>
                <div style={{ marginTop: 6, height: 22, borderRadius: 11, background: CREAM,
                  boxShadow: NEU_IN_SM, position: 'relative' }}>
                  <div style={{ position: 'absolute', left: '12%', right: '8%', top: 2, bottom: 2,
                    borderRadius: 9, background: 'linear-gradient(90deg, #f0bfa8, #c97a5d)' }}/>
                </div>
              </div>
            </NeuCard>

            <NeuCard padding={14} style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
                <Icon name="music" size={14} color="#4a4035"/>
                <div style={{ fontSize: 12.5, fontWeight: 600, color: '#2a221a' }}>Soundtrack</div>
                <div style={{ flex: 1 }}/>
                <div style={{ fontSize: 11, color: '#6e6358' }}>auto-ducked under voices</div>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: 10,
                borderRadius: 12, boxShadow: NEU_IN_SM }}>
                <div style={{ width: 36, height: 36, borderRadius: 8,
                  background: 'linear-gradient(135deg, #c4b8d8, #6a5a8a)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <Icon name="music" size={16} color="#fff"/>
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, fontWeight: 600, color: '#2a221a' }}>Slow Sundays</div>
                  <div style={{ fontSize: 11, color: '#6e6358' }}>warm · 84 bpm · 3:42</div>
                </div>
                <button aria-label="Replace track" style={{ width: 28, height: 28, borderRadius: 8,
                  border: 'none', background: CREAM, boxShadow: NEU_OUT_SM, cursor: 'pointer',
                  display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <Icon name="more" size={14} color="#4a4035"/>
                </button>
              </div>

              <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 6 }}>
                {[
                  { l: 'Crossfades',   on: true },
                  { l: 'Auto-rhythm to beat', on: true },
                  { l: 'Add captions',  on: false },
                ].map(o => (
                  <label key={o.l} style={{ display: 'flex', alignItems: 'center', gap: 10,
                    padding: '6px 4px', cursor: 'pointer' }}>
                    <div style={{ width: 28, height: 16, borderRadius: 8,
                      background: CREAM,
                      boxShadow: o.on ? 'inset 1px 1px 2.5px rgba(120,95,60,0.35), 0 0 0 1.5px #b86348' : NEU_IN_SM,
                      position: 'relative' }}>
                      <div style={{ position: 'absolute', top: 1.5, left: o.on ? 14 : 1.5,
                        width: 13, height: 13, borderRadius: 7,
                        background: o.on ? '#b86348' : '#cfc4b2',
                        transition: 'left 180ms' }}/>
                    </div>
                    <div style={{ fontSize: 12, color: '#2a221a' }}>{o.l}</div>
                  </label>
                ))}
              </div>
            </NeuCard>
          </div>
        </div>

        {/* Media bin */}
        <div style={{ flex: '0 0 auto' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
            <Icon name="film" size={14} color="#4a4035"/>
            <div style={{ fontSize: 12.5, fontWeight: 600, color: '#2a221a' }}>Bin · 23 clips</div>
            <NeuChip active>Smart</NeuChip>
            <NeuChip>By time</NeuChip>
            <NeuChip icon="people">With Henry</NeuChip>
          </div>
          <div style={{ padding: 10, borderRadius: 14, background: CREAM, boxShadow: NEU_IN_SM,
            display: 'flex', gap: 8, overflowX: 'auto' }}>
            {['lake','sunset','indoor','cake','garden','table','indoor','sunset','lake','garden','cake'].map((s,i) => (
              <div key={i} style={{ flexShrink: 0, position: 'relative', cursor: 'grab' }}>
                <Photo scene={s} w={94} h={62} round={8}/>
                <div style={{ position: 'absolute', bottom: 4, right: 4, padding: '1px 5px',
                  borderRadius: 4, background: 'rgba(0,0,0,0.6)', color: '#fff',
                  fontSize: 9.5, fontFamily: 'JetBrains Mono, monospace' }}>
                  {(2 + i * 0.7).toFixed(1)}s
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Timeline */}
        <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
            <Icon name="scissors" size={14} color="#4a4035"/>
            <div style={{ fontSize: 12.5, fontWeight: 600, color: '#2a221a' }}>Timeline</div>
            <div style={{ flex: 1 }}/>
            <div style={{ fontSize: 11, fontFamily: 'JetBrains Mono, monospace', color: '#6e6358' }}>
              5 clips · {totalDur.toFixed(1)}s
            </div>
          </div>

          <div style={{ flex: 1, padding: 14, borderRadius: 16, background: CREAM,
            boxShadow: NEU_IN_SM, display: 'flex', flexDirection: 'column', gap: 10, position: 'relative',
            minHeight: 200 }}>
            {/* Ruler */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 0, fontSize: 10,
              fontFamily: 'JetBrains Mono, monospace', color: '#8a7e70', position: 'relative' }}>
              {[0,5,10,15,20,25].map(t => (
                <div key={t} style={{ flex: 1, borderLeft: '1px solid rgba(120,95,60,0.18)',
                  paddingLeft: 4 }}>0:{t.toString().padStart(2,'0')}</div>
              ))}
            </div>

            {/* Video track */}
            <div style={{ display: 'flex', gap: 4, alignItems: 'stretch' }}>
              <div style={{ width: 50, fontSize: 10, fontWeight: 600, color: '#6e6358',
                textTransform: 'uppercase', letterSpacing: 0.6, alignSelf: 'center' }}>Video</div>
              <div style={{ flex: 1, display: 'flex', gap: 4, position: 'relative', height: 60 }}>
                {clips.map(c => (
                  <div key={c.id} style={{
                    flex: c.dur, position: 'relative',
                    borderRadius: 8, overflow: 'hidden',
                    boxShadow: c.selected ? `0 0 0 2.5px #b86348, ${NEU_OUT_SM}` : NEU_OUT_SM,
                  }}>
                    <Photo scene={c.scene} round={0}/>
                    <div style={{ position: 'absolute', left: 6, bottom: 4, color: '#fff',
                      fontSize: 10, fontWeight: 600, textShadow: '0 1px 2px rgba(0,0,0,0.6)' }}>
                      {c.label}
                    </div>
                  </div>
                ))}
                {/* Playhead */}
                <div style={{ position: 'absolute', top: -6, bottom: -18,
                  left: `${(playhead / totalDur) * 100}%`,
                  width: 2, background: '#b86348', pointerEvents: 'none' }}>
                  <div style={{ position: 'absolute', top: -4, left: -5, width: 12, height: 12,
                    borderRadius: 6, background: '#b86348',
                    boxShadow: '0 1px 3px rgba(40,30,20,0.4)' }}/>
                </div>
              </div>
            </div>

            {/* Audio track */}
            <div style={{ display: 'flex', gap: 4, alignItems: 'stretch' }}>
              <div style={{ width: 50, fontSize: 10, fontWeight: 600, color: '#6e6358',
                textTransform: 'uppercase', letterSpacing: 0.6, alignSelf: 'center' }}>Music</div>
              <div style={{ flex: 1, height: 32, borderRadius: 8,
                background: 'linear-gradient(135deg, #c4b8d8, #8a7eaa)',
                boxShadow: NEU_OUT_SM, display: 'flex', alignItems: 'center', padding: '0 12px' }}>
                {/* waveform */}
                <svg width="100%" height="20" viewBox="0 0 400 20" preserveAspectRatio="none">
                  {[...Array(80)].map((_,i) => {
                    const h = 4 + Math.abs(Math.sin(i * 0.7) * 8) + Math.abs(Math.cos(i * 0.3) * 4);
                    return <rect key={i} x={i * 5} y={(20-h)/2} width="2" height={h} fill="rgba(255,255,255,0.7)"/>;
                  })}
                </svg>
              </div>
            </div>

            {/* Title track */}
            <div style={{ display: 'flex', gap: 4, alignItems: 'stretch' }}>
              <div style={{ width: 50, fontSize: 10, fontWeight: 600, color: '#6e6358',
                textTransform: 'uppercase', letterSpacing: 0.6, alignSelf: 'center' }}>Title</div>
              <div style={{ flex: 1, height: 26, position: 'relative' }}>
                <div style={{ position: 'absolute', left: '4%', width: '22%', top: 0, bottom: 0,
                  borderRadius: 6, background: CREAM, boxShadow: NEU_OUT_SM,
                  display: 'flex', alignItems: 'center', padding: '0 8px',
                  fontSize: 10, fontWeight: 600, color: '#2a221a', gap: 4 }}>
                  <Icon name="text" size={11} color="#4a4035"/>
                  Henry, seventy-two
                </div>
                <div style={{ position: 'absolute', left: '74%', width: '18%', top: 0, bottom: 0,
                  borderRadius: 6, background: CREAM, boxShadow: NEU_OUT_SM,
                  display: 'flex', alignItems: 'center', padding: '0 8px',
                  fontSize: 10, fontWeight: 600, color: '#2a221a', gap: 4 }}>
                  <Icon name="text" size={11} color="#4a4035"/>
                  Thank you, Dad
                </div>
              </div>
            </div>

            <div style={{ flex: 1 }}/>

            {/* Timeline tools */}
            <div style={{ display: 'flex', gap: 6 }}>
              <NeuButton size="sm" icon="scissors">Split</NeuButton>
              <NeuButton size="sm" icon="transition">Transition</NeuButton>
              <NeuButton size="sm" icon="text">Title</NeuButton>
              <NeuButton size="sm" icon="wand">Auto-fit to song</NeuButton>
            </div>
          </div>
        </div>
      </div>
    </FeatureShell>
  );
}

window.MemoryEditor = MemoryEditor;
