// Family Room — Mantelpiece
// A paper-quiet Mac app whose hero is one horizontal river of time.

const SERIF = '"Source Serif 4", "Source Serif Pro", Georgia, serif';
const MONO  = '"JetBrains Mono", ui-monospace, monospace';

const PEOPLE = [
  { id: 'M', n: 'Margot', role: 'Grandma',   tone: '#b85a4a', born: '1953' },
  { id: 'H', n: 'Henry',  role: 'Grandpa',   tone: '#7a5a2a', born: '1953' },
  { id: 'C', n: 'Cora',   role: 'Mom',       tone: '#3e6a6a', born: '1986' },
  { id: 'T', n: 'Theo',   role: 'Dad',       tone: '#5a4a6a', born: '1984' },
  { id: 'I', n: 'Iris',   role: 'Daughter',  tone: '#a86a4a', born: '2018' },
];

// The river data — months across the year. Each entry is a "moment" with weight.
// Weight 1-5 controls the visual size of its plate on the river.
const MOMENTS = [
  { d: 'Jan 06', y: 2024, t: "First snow",                 w: 2, tone: '#9bb4c4', who: ['I','C'] },
  { d: 'Jan 14', y: 2024, t: "Sunday at Margot's",         w: 3, tone: '#b85a4a', who: ['M','H','C','I'] },
  { d: 'Feb 02', y: 2024, t: "Iris loses a tooth",         w: 4, tone: '#d4a06a', who: ['I'] },
  { d: 'Feb 18', y: 2024, t: "Theo's studio",              w: 2, tone: '#5a4a6a', who: ['T'] },
  { d: 'Mar 14', y: 2024, t: "Iris's first bike ride",     w: 5, tone: '#c87a4a', who: ['I','T'] },
  { d: 'Mar 28', y: 2024, t: "Cora's promotion",           w: 3, tone: '#3e6a6a', who: ['C','T'] },
  { d: 'Apr 09', y: 2024, t: "Garden, first crocus",       w: 1, tone: '#7a8a4a', who: ['M'] },
  { d: 'Apr 22', y: 2024, t: "Henry came home",            w: 5, tone: '#b85a4a', who: ['H','M','C'], pinned: true },
  { d: 'May 05', y: 2024, t: "Sunday picnic",              w: 3, tone: '#a8a04a', who: ['M','H','C','T','I'] },
  { d: 'May 19', y: 2024, t: "Margot's roses",             w: 2, tone: '#b85a4a', who: ['M'] },
  { d: 'Jun 02', y: 2024, t: "Lake house opening",         w: 4, tone: '#3e6a6a', who: ['M','H','C','T','I'] },
  { d: 'Jun 21', y: 2024, t: "Solstice",                   w: 2, tone: '#d4a06a', who: ['I'] },
  { d: 'Jul 04', y: 2024, t: "Backyard fireworks",         w: 4, tone: '#c84a4a', who: ['T','I'] },
  { d: 'Jul 18', y: 2024, t: "Iris reads aloud",           w: 3, tone: '#a86a4a', who: ['I','M'] },
  { d: 'Aug 09', y: 2024, t: "Henry's 72nd",               w: 5, tone: '#7a5a2a', who: ['H','M','C','T','I'], hero: true },
  { d: 'Aug 24', y: 2024, t: "Cabin trip",                 w: 3, tone: '#5a6a4a', who: ['C','T','I'] },
  { d: 'Sep 06', y: 2024, t: "Back to school",             w: 2, tone: '#a86a4a', who: ['I','C'] },
  { d: 'Sep 22', y: 2024, t: "Iris on stage",              w: 4, tone: '#9b5a6a', who: ['I'] },
  { d: 'Oct 11', y: 2024, t: "Apple picking",              w: 3, tone: '#c87a4a', who: ['M','C','I'] },
  { d: 'Oct 31', y: 2024, t: "A wolf, a witch, a ghost",   w: 4, tone: '#5a4a6a', who: ['I','T','C'] },
  { d: 'Nov 14', y: 2024, t: "Cora's grandmother died",    w: 3, tone: '#4a4036', who: ['C'] },
  { d: 'Nov 28', y: 2024, t: "Thanksgiving table",         w: 4, tone: '#b85a4a', who: ['M','H','C','T','I'] },
  { d: 'Dec 12', y: 2024, t: "Margot's last knit",         w: 2, tone: '#9bb4c4', who: ['M'] },
  { d: 'Dec 24', y: 2024, t: "Christmas Eve",              w: 5, tone: '#c84a4a', who: ['M','H','C','T','I'] },
];

// ─── Weight controls plate height. The river itself is a fixed band. ───
function plateHeight(w) {
  return [54, 80, 112, 150, 196][w - 1] || 80;
}
function plateWidth(w) {
  return [40, 56, 80, 110, 140][w - 1] || 56;
}

// ─── The Photo plate ───
// Visually: a soft-edged colored field with subtle scan-of-paper look.
// Crucially: no fake border, no rounded corners — this is a museum print, not a card.
function PlatePhoto({ tone, label }) {
  return (
    <div style={{
      position: 'relative', width: '100%', height: '100%', overflow: 'hidden',
      background: `linear-gradient(165deg, ${tone}ee 0%, ${tone}cc 50%, ${tone}88 100%)`,
      boxShadow: 'inset 0 0 0 0.5px rgba(20,15,10,0.25), inset 0 -8px 20px rgba(20,15,10,0.18)',
    }}>
      {/* faint paper grain */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', mixBlendMode: 'multiply', opacity: 0.18 }}>
        <filter id="g">
          <feTurbulence type="fractalNoise" baseFrequency="1.6" numOctaves="1" seed="3" />
          <feColorMatrix values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 0.6 0"/>
        </filter>
        <rect width="100%" height="100%" filter="url(#g)" />
      </svg>
      {/* hairline highlight from the upper-left "window" */}
      <div style={{
        position: 'absolute', inset: 0,
        background: 'linear-gradient(160deg, rgba(255,245,220,0.18) 0%, transparent 35%)',
      }} />
    </div>
  );
}

// ─── A Person (avatar — initial in a paper circle, soft tone) ───
function Person({ p, on = false, dim = false, onClick, size = 44 }) {
  return (
    <div onClick={onClick} style={{
      display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, cursor: 'pointer',
      opacity: dim ? 0.32 : 1, transition: 'opacity 220ms ease',
    }}>
      <div style={{
        width: size, height: size, borderRadius: '50%',
        background: p.tone,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        color: '#f7f4ee',
        fontFamily: SERIF, fontWeight: 400, fontSize: size * 0.4,
        fontStyle: 'italic', letterSpacing: -0.5,
        boxShadow: on
          ? `0 0 0 1px rgba(40,30,20,0.6), 0 0 0 4px rgba(247,244,238,1), 0 0 0 5px ${p.tone}, inset 0 -3px 6px rgba(0,0,0,0.18)`
          : 'inset 0 -3px 6px rgba(0,0,0,0.18), 0 1px 2px rgba(40,30,20,0.18)',
        transition: 'box-shadow 220ms ease',
      }}>{p.id}</div>
      <div style={{
        fontFamily: MONO, fontSize: 9, fontWeight: 500, letterSpacing: 1.4,
        textTransform: 'uppercase', color: on ? '#1a1612' : '#8a7e6e',
      }}>{p.n}</div>
    </div>
  );
}

// ─── The People rail — fixed left column, always visible ───
function PeopleRail({ selected, onSelect }) {
  return (
    <div style={{
      width: 92, flexShrink: 0,
      display: 'flex', flexDirection: 'column',
      alignItems: 'center', padding: '24px 0',
      borderRight: '0.5px solid rgba(40,30,20,0.12)',
    }}>
      <div style={{
        fontFamily: MONO, fontSize: 9, fontWeight: 500, letterSpacing: 2,
        textTransform: 'uppercase', color: '#8a7e6e', marginBottom: 22,
      }}>Five</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 18, flex: 1 }}>
        {PEOPLE.map(p => (
          <Person
            key={p.id}
            p={p}
            on={selected.includes(p.id)}
            dim={selected.length > 0 && !selected.includes(p.id)}
            onClick={() => onSelect(p.id)}
          />
        ))}
      </div>
      <div style={{ height: 0.5, width: 24, background: 'rgba(40,30,20,0.18)', marginTop: 12, marginBottom: 12 }} />
      <div style={{
        fontFamily: MONO, fontSize: 8.5, fontWeight: 500, letterSpacing: 1.6,
        textAlign: 'center', color: '#a89c8c', lineHeight: 1.6,
      }}>tap to<br/>filter</div>
    </div>
  );
}

// ─── The river of time ───
function River({ selected = [], expanded = null }) {
  // baseline along which plates sit, anchored to bottom
  return (
    <div style={{
      position: 'relative', width: '100%', height: 320,
      paddingTop: 8, paddingBottom: 36,
    }}>
      {/* the year tick line, very thin */}
      <div style={{
        position: 'absolute', left: 12, right: 12, bottom: 30,
        height: 0.5, background: 'rgba(40,30,20,0.22)',
      }} />

      {/* month tick marks */}
      <div style={{
        position: 'absolute', left: 12, right: 12, bottom: 12, height: 18,
        display: 'flex', justifyContent: 'space-between',
      }}>
        {['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'].map((m, i) => (
          <div key={m} style={{
            display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 5,
          }}>
            <div style={{ width: 0.5, height: 5, background: 'rgba(40,30,20,0.4)' }} />
            <div style={{
              fontFamily: MONO, fontSize: 9.5, fontWeight: 500, letterSpacing: 1.4,
              textTransform: 'uppercase',
              color: i === 7 ? '#1a1612' : '#8a7e6e',
            }}>{m}</div>
          </div>
        ))}
      </div>

      {/* plates */}
      <div style={{
        position: 'absolute', inset: '8px 12px 50px',
        display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between',
      }}>
        {MOMENTS.map((m, i) => {
          const active = selected.length === 0 || m.who.some(id => selected.includes(id));
          const opacity = active ? 1 : 0.18;
          const h = plateHeight(m.w);
          const w = plateWidth(m.w);
          return (
            <div key={i} style={{
              position: 'relative',
              width: w, height: h,
              opacity, transition: 'opacity 280ms ease',
              cursor: 'pointer',
            }}>
              <PlatePhoto tone={m.tone} />
              {/* hero label — only on the heaviest plates */}
              {m.w >= 4 && (
                <div style={{
                  position: 'absolute', top: -28, left: 0,
                  fontFamily: SERIF, fontSize: 12, fontStyle: 'italic',
                  fontWeight: 400, color: '#1a1612', whiteSpace: 'nowrap',
                  letterSpacing: -0.1, lineHeight: 1.1,
                }}>
                  {m.t}
                </div>
              )}
              {/* tiny date pin under each plate */}
              <div style={{
                position: 'absolute', bottom: -16, left: '50%', transform: 'translateX(-50%)',
                fontFamily: MONO, fontSize: 8, fontWeight: 500, letterSpacing: 0.8,
                textTransform: 'uppercase', color: m.hero ? '#1a1612' : '#a89c8c',
                whiteSpace: 'nowrap',
              }}>{m.d.split(' ')[1]}</div>
              {/* hero star — a small filled circle to mark the heaviest */}
              {m.hero && (
                <div style={{
                  position: 'absolute', top: -38, left: 0,
                  width: 5, height: 5, borderRadius: '50%', background: '#1a1612',
                }} />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ─── Mac window chrome — restrained, paper-on-paper ───
function MacChrome({ width = 1280, height = 800, children, title }) {
  return (
    <div style={{
      width, height, flexShrink: 0,
      borderRadius: 10, overflow: 'hidden', position: 'relative',
      background: '#f7f4ee',
      boxShadow:
        '0 0 0 0.5px rgba(40,30,20,0.18), ' +
        '0 1px 2px rgba(40,30,20,0.06), ' +
        '0 30px 60px -20px rgba(40,30,20,0.22)',
      display: 'flex', flexDirection: 'column',
      fontFamily: SERIF,
    }}>
      <div style={{
        height: 32, display: 'flex', alignItems: 'center', padding: '0 14px',
        flexShrink: 0,
        borderBottom: '0.5px solid rgba(40,30,20,0.1)',
      }}>
        <div style={{ display: 'flex', gap: 8 }}>
          {['#dcd0bc','#dcd0bc','#dcd0bc'].map((c, i) => (
            <div key={i} style={{
              width: 11, height: 11, borderRadius: 6, background: c,
              boxShadow: 'inset 0 0 0 0.5px rgba(40,30,20,0.18)',
            }} />
          ))}
        </div>
        <div style={{ flex: 1, textAlign: 'center',
          fontFamily: MONO, fontSize: 10, fontWeight: 500, letterSpacing: 1.6,
          textTransform: 'uppercase', color: '#8a7e6e',
        }}>{title}</div>
      </div>
      {children}
    </div>
  );
}

// ─── The mantelpiece (main view) ───
function Mantelpiece() {
  return (
    <MacChrome title="Family Room — The Alderhouse · 4,218">
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <PeopleRail selected={['C']} onSelect={() => {}} />

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          {/* Search bar — one sentence, no chrome */}
          <div style={{
            padding: '20px 36px 0', display: 'flex', alignItems: 'baseline', gap: 16,
          }}>
            <div style={{
              fontFamily: MONO, fontSize: 10, fontWeight: 500, letterSpacing: 2,
              textTransform: 'uppercase', color: '#8a7e6e',
            }}>Find</div>
            <input
              defaultValue="Cora, this year"
              style={{
                flex: 1, border: 'none', outline: 'none', background: 'transparent',
                fontFamily: SERIF, fontSize: 22, fontStyle: 'italic',
                fontWeight: 300, color: '#1a1612', letterSpacing: -0.3,
                borderBottom: '0.5px solid rgba(40,30,20,0.22)',
                paddingBottom: 6,
              }}
            />
            <div style={{
              fontFamily: MONO, fontSize: 10, fontWeight: 500, letterSpacing: 1.4,
              textTransform: 'uppercase', color: '#8a7e6e',
            }}>14 of 4,218</div>
          </div>

          {/* Editorial header for the river */}
          <div style={{
            padding: '52px 36px 0',
          }}>
            <div style={{
              fontFamily: MONO, fontSize: 10, fontWeight: 500, letterSpacing: 2.4,
              textTransform: 'uppercase', color: '#8a7e6e',
            }}>The year, weighted</div>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 14, marginTop: 6 }}>
              <div style={{
                fontFamily: SERIF, fontSize: 64, fontWeight: 300,
                letterSpacing: -2, lineHeight: 1, color: '#1a1612',
                fontVariationSettings: '"opsz" 60',
              }}>2024</div>
              <div style={{
                fontFamily: SERIF, fontSize: 19, fontStyle: 'italic',
                color: '#4a4036', fontWeight: 400, paddingBottom: 6,
              }}>— a slow spring, a loud summer, a quiet end.</div>
            </div>
          </div>

          {/* The river itself */}
          <div style={{ padding: '24px 24px 0', flex: 1, minHeight: 0 }}>
            <River selected={['C']} />
          </div>

          {/* On this day footer */}
          <div style={{
            padding: '18px 36px 22px',
            borderTop: '0.5px solid rgba(40,30,20,0.1)',
            display: 'flex', alignItems: 'center', gap: 18,
          }}>
            <div style={{
              fontFamily: MONO, fontSize: 9.5, fontWeight: 500, letterSpacing: 2.2,
              textTransform: 'uppercase', color: '#8a7e6e', flexShrink: 0,
            }}>On this day</div>
            <div style={{ flex: 1, display: 'flex', gap: 18, alignItems: 'center' }}>
              {[
                { y: 2023, tone: '#9b5a6a', t: 'Margot baked alone' },
                { y: 2022, tone: '#7a8a4a', t: 'Iris in the garden' },
                { y: 2020, tone: '#5a4a6a', t: 'Theo in his studio' },
              ].map((d, i) => (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <div style={{ width: 32, height: 32 }}>
                    <PlatePhoto tone={d.tone} />
                  </div>
                  <div>
                    <div style={{ fontFamily: MONO, fontSize: 9, fontWeight: 500, letterSpacing: 1.2,
                      textTransform: 'uppercase', color: '#a89c8c' }}>{d.y}</div>
                    <div style={{ fontFamily: SERIF, fontSize: 13.5, fontStyle: 'italic',
                      fontWeight: 400, color: '#1a1612', marginTop: 1 }}>{d.t}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </MacChrome>
  );
}

// ─── A moment, opened ───
function MomentOpened() {
  // Henry's 72nd, opened in editorial layout
  const HERO = MOMENTS.find(m => m.hero);
  return (
    <MacChrome title="Family Room — A Moment">
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <PeopleRail selected={['H','M','C','T','I']} onSelect={() => {}} />

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
          {/* Collapsed scrubber */}
          <div style={{
            padding: '14px 36px',
            borderBottom: '0.5px solid rgba(40,30,20,0.1)',
            display: 'flex', alignItems: 'center', gap: 14,
          }}>
            <div style={{ fontFamily: MONO, fontSize: 9.5, fontWeight: 500,
              letterSpacing: 1.6, color: '#8a7e6e', textTransform: 'uppercase' }}>2024</div>
            <div style={{ flex: 1, height: 1, background: 'rgba(40,30,20,0.12)', position: 'relative' }}>
              {MOMENTS.map((m, i) => (
                <div key={i} style={{
                  position: 'absolute', top: -3, left: `${(i / (MOMENTS.length - 1)) * 100}%`,
                  width: 4, height: 4, borderRadius: '50%',
                  background: m.hero ? '#1a1612' : 'rgba(40,30,20,0.3)',
                  transform: 'translateX(-50%)',
                }} />
              ))}
              <div style={{
                position: 'absolute', top: -7, left: `${(14 / (MOMENTS.length - 1)) * 100}%`,
                width: 12, height: 12, borderRadius: '50%',
                background: '#7a5a2a', boxShadow: '0 0 0 3px #f7f4ee, 0 0 0 4px #7a5a2a',
                transform: 'translateX(-50%)',
              }} />
            </div>
            <div style={{ fontFamily: MONO, fontSize: 9.5, fontWeight: 500, letterSpacing: 1.6,
              color: '#1a1612', textTransform: 'uppercase' }}>Aug 09</div>
            <div style={{ fontFamily: MONO, fontSize: 9.5, fontWeight: 500, letterSpacing: 1.6,
              color: '#8a7e6e', textTransform: 'uppercase' }}>esc ⤴</div>
          </div>

          {/* Editorial spread */}
          <div style={{ flex: 1, overflow: 'auto', padding: '36px 56px 36px' }}>
            <div style={{ fontFamily: MONO, fontSize: 10, fontWeight: 500, letterSpacing: 2.4,
              textTransform: 'uppercase', color: '#8a7e6e' }}>August 9 · 2024 · The Lake House</div>
            <h2 style={{ fontFamily: SERIF, fontSize: 56, fontWeight: 300, letterSpacing: -1.5,
              margin: '8px 0 4px', lineHeight: 1, color: '#1a1612',
              fontVariationSettings: '"opsz" 60' }}>Henry's 72nd</h2>
            <div style={{ fontFamily: SERIF, fontSize: 19, fontStyle: 'italic',
              color: '#4a4036', fontWeight: 400, maxWidth: 580 }}>
              We took the boat out at six. The sky was the color of a peach pit. Iris fell asleep on his shoulder.
            </div>

            <div style={{
              display: 'grid', gridTemplateColumns: '2fr 1fr 1fr',
              gridAutoRows: '180px', gap: 12, marginTop: 30,
            }}>
              <div style={{ gridRow: 'span 2' }}><PlatePhoto tone="#7a5a2a" /></div>
              <div><PlatePhoto tone="#b85a4a" /></div>
              <div><PlatePhoto tone="#3e6a6a" /></div>
              <div><PlatePhoto tone="#c87a4a" /></div>
              <div><PlatePhoto tone="#a8a04a" /></div>
            </div>

            <div style={{
              marginTop: 28, display: 'flex', gap: 36,
              fontFamily: MONO, fontSize: 10.5, fontWeight: 500, letterSpacing: 1.4,
              textTransform: 'uppercase', color: '#8a7e6e',
            }}>
              <div><span style={{ color: '#1a1612' }}>61</span> &nbsp;&nbsp;Photos</div>
              <div><span style={{ color: '#1a1612' }}>3</span> &nbsp;&nbsp;Videos</div>
              <div><span style={{ color: '#1a1612' }}>5</span> &nbsp;&nbsp;Voices</div>
              <div><span style={{ color: '#1a1612' }}>★ 5.0</span> &nbsp;&nbsp;Family weight</div>
            </div>
          </div>
        </div>
      </div>
    </MacChrome>
  );
}

// ─── Search view — query reshaping the river ───
function SearchView() {
  return (
    <MacChrome title="Family Room — Find">
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <PeopleRail selected={['I']} onSelect={() => {}} />

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <div style={{
            padding: '32px 56px 0',
          }}>
            <div style={{ fontFamily: MONO, fontSize: 10, fontWeight: 500, letterSpacing: 2.4,
              textTransform: 'uppercase', color: '#8a7e6e' }}>Searching</div>
            <div style={{
              marginTop: 10, display: 'flex', alignItems: 'baseline', gap: 14,
              borderBottom: '0.5px solid rgba(40,30,20,0.22)', paddingBottom: 14,
            }}>
              <div style={{
                fontFamily: SERIF, fontSize: 38, fontStyle: 'italic',
                fontWeight: 300, color: '#1a1612', letterSpacing: -0.8, lineHeight: 1.1,
                fontVariationSettings: '"opsz" 48',
              }}>Iris at the lake house, summer</div>
              <div style={{
                width: 2, height: 30, background: '#1a1612',
                animation: 'blink 1s steps(2) infinite',
              }} />
            </div>

            <div style={{ marginTop: 14, display: 'flex', gap: 22,
              fontFamily: MONO, fontSize: 10, fontWeight: 500, letterSpacing: 1.6,
              textTransform: 'uppercase', color: '#8a7e6e' }}>
              <div><span style={{ color: '#1a1612' }}>iris</span> · person</div>
              <div><span style={{ color: '#1a1612' }}>lake house</span> · place</div>
              <div><span style={{ color: '#1a1612' }}>jun–aug</span> · season</div>
              <div style={{ color: '#a89c8c' }}>— 7 moments, 88 photos</div>
            </div>
          </div>

          <div style={{ padding: '40px 24px 0', flex: 1, minHeight: 0, position: 'relative' }}>
            {/* a cleaner river — only the matched moments are visible, others are ghost dots */}
            <div style={{ position: 'relative', height: 280, width: '100%' }}>
              <div style={{ position: 'absolute', left: 12, right: 12, bottom: 30,
                height: 0.5, background: 'rgba(40,30,20,0.22)' }} />
              <div style={{
                position: 'absolute', left: 12, right: 12, bottom: 12, height: 18,
                display: 'flex', justifyContent: 'space-between',
              }}>
                {['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'].map((m, i) => (
                  <div key={m} style={{
                    display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 5,
                  }}>
                    <div style={{ width: 0.5, height: 5, background: 'rgba(40,30,20,0.4)' }} />
                    <div style={{
                      fontFamily: MONO, fontSize: 9.5, fontWeight: 500, letterSpacing: 1.4,
                      textTransform: 'uppercase',
                      color: (i >= 5 && i <= 7) ? '#1a1612' : 'rgba(138,126,110,0.4)',
                    }}>{m}</div>
                  </div>
                ))}
              </div>
              <div style={{
                position: 'absolute', inset: '8px 12px 50px',
                display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between',
              }}>
                {MOMENTS.map((m, i) => {
                  const matched = m.who.includes('I') && (i >= 10 && i <= 15);
                  if (!matched) {
                    return (
                      <div key={i} style={{
                        width: 4, height: 4, borderRadius: '50%',
                        background: 'rgba(40,30,20,0.18)',
                        marginBottom: 4,
                      }} />
                    );
                  }
                  const h = plateHeight(m.w);
                  const w = plateWidth(m.w);
                  return (
                    <div key={i} style={{ position: 'relative', width: w, height: h }}>
                      <PlatePhoto tone={m.tone} />
                      <div style={{
                        position: 'absolute', top: -28, left: 0,
                        fontFamily: SERIF, fontSize: 12, fontStyle: 'italic',
                        fontWeight: 400, color: '#1a1612', whiteSpace: 'nowrap',
                      }}>{m.t}</div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>

          <div style={{
            padding: '14px 56px 22px',
            borderTop: '0.5px solid rgba(40,30,20,0.1)',
            fontFamily: MONO, fontSize: 9.5, fontWeight: 500, letterSpacing: 1.6,
            textTransform: 'uppercase', color: '#8a7e6e',
          }}>
            ⏎  Open the matched stretch &nbsp;·&nbsp;  ⌘K  Refine &nbsp;·&nbsp;  Esc  Back to mantelpiece
          </div>
        </div>
      </div>
    </MacChrome>
  );
}

window.Mantelpiece = Mantelpiece;
window.MomentOpened = MomentOpened;
window.SearchView = SearchView;
