// Mac — DVD MENU energy. Late-90s/early-2000s home theater.
// Deep midnight chrome, neon magenta + cyan + lime, glossy reflective tiles,
// chunky beveled controls, "SCENE SELECTION" / "PLAY MOVIE" affordances.

const DVD_PEOPLE = [
  { n: 'Margot', initial: 'M', color: '#ff3a8c', shadow: '#5a0a30', role: 'Grandma' },
  { n: 'Henry',  initial: 'H', color: '#3aff9c', shadow: '#0a4a28', role: 'Grandpa' },
  { n: 'Cora',   initial: 'C', color: '#3ac8ff', shadow: '#0a3a5a', role: 'Mom' },
  { n: 'Theo',   initial: 'T', color: '#ffcc28', shadow: '#5a3e08', role: 'Dad' },
  { n: 'Iris',   initial: 'I', color: '#c83aff', shadow: '#3a0a5a', role: 'Daughter' },
];

const DVD_YEARS = [
  { y: 2026, count: 218, hue: '#ff3a8c', label: 'NEW' },
  { y: 2025, count: 942, hue: '#3aff9c' },
  { y: 2024, count: 786, hue: '#3ac8ff' },
  { y: 2023, count: 654, hue: '#ffcc28' },
];

// Photo replacement that respects DVD palette — saturated, slightly grainy
function DvdPhoto({ hue = '#ff3a8c', deep = '#1a0a2a', label, ratio = 1, style = {} }) {
  return (
    <div style={{
      position: 'relative', width: '100%', aspectRatio: ratio, overflow: 'hidden',
      background: `radial-gradient(ellipse at 28% 22%, ${hue}cc 0%, ${hue}88 30%, ${deep} 90%)`,
      ...style,
    }}>
      {/* scanline overlay — adds CRT TV feel */}
      <div style={{
        position: 'absolute', inset: 0,
        backgroundImage: 'repeating-linear-gradient(0deg, rgba(0,0,0,0.18) 0px, rgba(0,0,0,0.18) 1px, transparent 1px, transparent 3px)',
        mixBlendMode: 'multiply', pointerEvents: 'none',
      }} />
      {/* shine */}
      <div style={{
        position: 'absolute', inset: 0,
        background: 'linear-gradient(135deg, rgba(255,255,255,0.25) 0%, transparent 35%, transparent 65%, rgba(255,255,255,0.08) 100%)',
        mixBlendMode: 'screen', pointerEvents: 'none',
      }} />
      {label && (
        <div style={{
          position: 'absolute', bottom: 6, left: 8,
          fontFamily: '"Courier New", ui-monospace, monospace',
          fontSize: 9, fontWeight: 700, color: 'rgba(255,255,255,0.85)',
          letterSpacing: 1.2, textTransform: 'uppercase',
          textShadow: '0 1px 2px rgba(0,0,0,0.6)',
        }}>{label}</div>
      )}
    </div>
  );
}

// ─── DVD-style chrome tile — beveled, glossy, looks like a button on a remote ───
function ChromeTile({ children, accent = '#ff3a8c', selected = false, onClick, style = {} }) {
  return (
    <div onClick={onClick} style={{
      position: 'relative', borderRadius: 8,
      background: selected
        ? `linear-gradient(180deg, ${accent}40 0%, ${accent}25 50%, ${accent}50 100%)`
        : 'linear-gradient(180deg, #2a1a4a 0%, #1a0e36 50%, #100828 100%)',
      boxShadow: selected
        ? `inset 0 1px 0 ${accent}cc, inset 0 -1px 0 rgba(0,0,0,0.5), 0 0 0 1.5px ${accent}, 0 0 18px ${accent}88, 0 4px 10px rgba(0,0,0,0.5)`
        : 'inset 0 1px 0 rgba(180,140,255,0.25), inset 0 -1px 0 rgba(0,0,0,0.55), 0 0 0 0.5px rgba(140,100,200,0.4), 0 4px 8px rgba(0,0,0,0.45)',
      cursor: 'pointer',
      transition: 'all 200ms cubic-bezier(.2,.9,.3,1.2)',
      ...style,
    }}>{children}</div>
  );
}

// ─── DVD year tile — rectangular, glowing, with movie-cover aesthetics ───
function DvdYearTile({ year, selected = false }) {
  return (
    <ChromeTile accent={year.hue} selected={selected} style={{ width: 124, height: 158, padding: 5 }}>
      {/* Cover thumbnail */}
      <div style={{
        position: 'relative', width: '100%', height: 96, borderRadius: 4, overflow: 'hidden',
        boxShadow: 'inset 0 0 0 0.5px rgba(0,0,0,0.5), inset 0 0 18px rgba(0,0,0,0.4)',
      }}>
        <DvdPhoto hue={year.hue} ratio={1.3} />
        {/* center badge */}
        <div style={{
          position: 'absolute', top: '50%', left: '50%',
          transform: 'translate(-50%, -50%)',
          fontFamily: '"Helvetica Neue", Arial, sans-serif', fontWeight: 900,
          fontSize: 28, color: '#fff',
          textShadow: `0 2px 0 ${year.hue}, 0 0 14px rgba(0,0,0,0.6), 0 0 4px rgba(0,0,0,0.8)`,
          letterSpacing: -1,
        }}>{year.y}</div>
        {year.label && (
          <div style={{
            position: 'absolute', top: 5, right: 5,
            background: year.hue, color: '#0a0420',
            fontFamily: '"Helvetica Neue", sans-serif', fontWeight: 900,
            fontSize: 8, padding: '2px 5px', letterSpacing: 0.8,
            boxShadow: `0 0 10px ${year.hue}cc`,
          }}>{year.label}</div>
        )}
      </div>
      {/* meta */}
      <div style={{
        marginTop: 8, paddingLeft: 4,
        fontFamily: '"Helvetica Neue", Arial, sans-serif',
      }}>
        <div style={{
          fontSize: 9, fontWeight: 700, letterSpacing: 1.4, textTransform: 'uppercase',
          color: selected ? year.hue : 'rgba(220,210,255,0.55)',
          textShadow: selected ? `0 0 6px ${year.hue}` : 'none',
        }}>Disc · {year.count}</div>
        <div style={{
          fontSize: 11, fontWeight: 700, marginTop: 1,
          color: selected ? '#fff' : 'rgba(230,225,255,0.85)',
        }}>Memories</div>
      </div>
      {/* selected chevron / play indicator */}
      {selected && (
        <div style={{
          position: 'absolute', top: 8, left: 8,
          width: 0, height: 0,
          borderLeft: `8px solid ${year.hue}`,
          borderTop: '5px solid transparent',
          borderBottom: '5px solid transparent',
          filter: `drop-shadow(0 0 4px ${year.hue})`,
        }} />
      )}
    </ChromeTile>
  );
}

// ─── DVD library tile — looks like a movie chapter poster ───
function DvdLibraryTile({ hue, label, sub, selected = false, big = false }) {
  return (
    <ChromeTile accent={hue} selected={selected} style={{
      width: '100%', height: '100%', padding: 4,
    }}>
      <div style={{
        position: 'relative', width: '100%', height: '100%', borderRadius: 4, overflow: 'hidden',
        boxShadow: 'inset 0 0 0 0.5px rgba(0,0,0,0.5), inset 0 0 24px rgba(0,0,0,0.5)',
      }}>
        <DvdPhoto hue={hue} ratio={1.4} />
        {/* gradient ramp at bottom for label legibility */}
        <div style={{
          position: 'absolute', left: 0, right: 0, bottom: 0, height: '50%',
          background: 'linear-gradient(180deg, transparent 0%, rgba(10,4,30,0.85) 80%, rgba(10,4,30,0.95) 100%)',
        }} />
        {/* title */}
        <div style={{
          position: 'absolute', left: 8, right: 8, bottom: 6,
        }}>
          <div style={{
            fontFamily: '"Helvetica Neue", Arial, sans-serif',
            fontSize: big ? 9 : 8, fontWeight: 700, letterSpacing: 1.6, textTransform: 'uppercase',
            color: hue, textShadow: `0 0 6px ${hue}aa`,
          }}>Chapter</div>
          <div style={{
            fontFamily: '"Helvetica Neue", Arial, sans-serif',
            fontSize: big ? 16 : 13, fontWeight: 800, color: '#fff',
            lineHeight: 1.1, letterSpacing: -0.3,
            textShadow: '0 1px 2px rgba(0,0,0,0.7)',
          }}>{label}</div>
          {sub && (
            <div style={{
              fontFamily: '"Helvetica Neue", Arial, sans-serif',
              fontSize: 9, fontWeight: 500, marginTop: 2,
              color: 'rgba(220,210,255,0.7)',
            }}>{sub}</div>
          )}
        </div>
        {/* selected play triangle */}
        {selected && (
          <div style={{
            position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)',
            width: 0, height: 0,
            borderLeft: '20px solid #fff',
            borderTop: '13px solid transparent',
            borderBottom: '13px solid transparent',
            filter: `drop-shadow(0 0 12px ${hue}) drop-shadow(0 0 4px rgba(0,0,0,0.6))`,
          }} />
        )}
      </div>
    </ChromeTile>
  );
}

// ─── Family member dot — DVD remote button style ───
function DvdPersonDot({ p, selected = false }) {
  const size = selected ? 56 : 48;
  return (
    <div style={{
      display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 7,
      cursor: 'pointer', position: 'relative',
    }}>
      <div style={{
        width: size, height: size, borderRadius: '50%',
        background: `radial-gradient(circle at 32% 26%, ${p.color}ff 0%, ${p.color} 38%, ${p.shadow} 100%)`,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        color: '#0a0420',
        fontFamily: '"Helvetica Neue", Arial, sans-serif',
        fontSize: size * 0.42, fontWeight: 900,
        boxShadow:
          'inset 0 2px 4px rgba(255,255,255,0.55), ' +
          'inset 0 -3px 6px rgba(0,0,0,0.4), ' +
          `0 0 ${selected ? 22 : 12}px ${p.color}${selected ? 'cc' : '66'}, ` +
          `0 4px 8px rgba(0,0,0,0.55)` +
          (selected ? `, 0 0 0 2.5px rgba(255,255,255,0.95), 0 0 0 4.5px ${p.color}` : ''),
        transition: 'all 200ms cubic-bezier(.2,.9,.3,1.2)',
        textShadow: '0 1px 0 rgba(255,255,255,0.4)',
      }}>{p.initial}</div>
      <div style={{
        fontFamily: '"Helvetica Neue", Arial, sans-serif',
        fontSize: 9.5, fontWeight: 700, letterSpacing: 1.4, textTransform: 'uppercase',
        color: selected ? p.color : 'rgba(220,210,255,0.7)',
        textShadow: selected ? `0 0 8px ${p.color}aa` : 'none',
      }}>{p.n}</div>
    </div>
  );
}

// ─── A flickering / scrolling marquee strip in DVD style ───
function DvdMarquee({ text, hue = '#3ac8ff' }) {
  return (
    <div style={{
      overflow: 'hidden', position: 'relative',
      background: 'linear-gradient(180deg, #0a0420 0%, #15082e 100%)',
      borderTop: `0.5px solid ${hue}55`, borderBottom: `0.5px solid ${hue}55`,
      padding: '4px 0',
      boxShadow: `inset 0 1px 0 ${hue}22, inset 0 -1px 0 ${hue}22`,
    }}>
      <div style={{
        whiteSpace: 'nowrap',
        fontFamily: '"Courier New", monospace',
        fontSize: 10, fontWeight: 700, color: hue,
        letterSpacing: 2.2, textTransform: 'uppercase',
        textShadow: `0 0 6px ${hue}aa`,
      }}>{text}</div>
    </div>
  );
}

function MacRoomFrame({ width = 1300, height = 840, children }) {
  return (
    <div style={{
      width, height, flexShrink: 0, borderRadius: 14, overflow: 'hidden', position: 'relative',
      boxShadow: '0 36px 80px rgba(10,4,30,0.55), 0 0 0 0.5px rgba(0,0,0,0.45), 0 0 80px rgba(58,200,255,0.08)',
    }}>{children}</div>
  );
}

function MacRoomApp({ selectedYear = 2026, selectedPerson = 'Iris' }) {
  return (
    <div style={{
      position: 'relative', width: '100%', height: '100%',
      overflow: 'hidden',
      display: 'flex', flexDirection: 'column',
      // The DVD menu background — deep midnight with starfield + soft purple glow
      background: '#0a0420',
      backgroundImage: `
        radial-gradient(ellipse 60% 45% at 80% 15%, rgba(255,58,140,0.18) 0%, transparent 60%),
        radial-gradient(ellipse 70% 50% at 15% 85%, rgba(58,200,255,0.16) 0%, transparent 60%),
        radial-gradient(ellipse 50% 40% at 50% 60%, rgba(200,58,255,0.10) 0%, transparent 70%),
        linear-gradient(180deg, #0e0628 0%, #0a0420 50%, #060218 100%)
      `,
      fontFamily: '"Helvetica Neue", Arial, sans-serif',
    }}>
      {/* faint starfield */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.5, pointerEvents: 'none' }}>
        <defs>
          <radialGradient id="starg">
            <stop offset="0%" stopColor="#fff" stopOpacity="0.9"/>
            <stop offset="100%" stopColor="#fff" stopOpacity="0"/>
          </radialGradient>
        </defs>
        {Array.from({ length: 60 }).map((_, i) => {
          const x = (i * 73) % 1300;
          const y = (i * 137) % 840;
          const r = (i % 4) * 0.3 + 0.4;
          return <circle key={i} cx={x} cy={y} r={r} fill="url(#starg)" />;
        })}
      </svg>

      {/* CRT scanlines over everything */}
      <div style={{
        position: 'absolute', inset: 0, pointerEvents: 'none', zIndex: 50,
        backgroundImage: 'repeating-linear-gradient(0deg, rgba(255,255,255,0.025) 0px, rgba(255,255,255,0.025) 1px, transparent 1px, transparent 3px)',
        mixBlendMode: 'screen',
      }} />

      {/* Subtle vignette */}
      <div style={{
        position: 'absolute', inset: 0, pointerEvents: 'none', zIndex: 49,
        background: 'radial-gradient(ellipse 100% 80% at 50% 50%, transparent 50%, rgba(0,0,0,0.55) 100%)',
      }} />

      {/* Titlebar — traffic lights on dark */}
      <div style={{
        position: 'relative', zIndex: 10,
        height: 36, display: 'flex', alignItems: 'center',
        padding: '0 14px', flexShrink: 0,
        background: 'linear-gradient(180deg, rgba(255,255,255,0.04) 0%, rgba(0,0,0,0.2) 100%)',
        borderBottom: '0.5px solid rgba(140,100,200,0.25)',
      }}>
        <div style={{ display: 'flex', gap: 8 }}>
          {['#ff5f57','#febc2e','#28c840'].map((c, i) => (
            <div key={i} style={{
              width: 12, height: 12, borderRadius: 6, background: c,
              boxShadow: 'inset 0 0 0 0.5px rgba(0,0,0,0.3), inset 0 1px 0 rgba(255,255,255,0.4), 0 0 4px rgba(0,0,0,0.4)',
            }} />
          ))}
        </div>
        <div style={{
          flex: 1, textAlign: 'center',
          fontSize: 10.5, fontWeight: 700, letterSpacing: 2.4,
          color: 'rgba(180,160,230,0.6)', textTransform: 'uppercase',
        }}>Family Room — Disc 01 · The Alderhouse</div>
        {/* time / mode lozenge */}
        <div style={{
          fontFamily: '"Courier New", monospace', fontSize: 10, fontWeight: 700,
          color: '#3aff9c', letterSpacing: 1.4,
          padding: '3px 8px', borderRadius: 3,
          background: 'rgba(58,255,156,0.1)',
          boxShadow: 'inset 0 0 0 0.5px rgba(58,255,156,0.4)',
          textShadow: '0 0 6px rgba(58,255,156,0.6)',
        }}>● REC</div>
      </div>

      {/* Main content */}
      <div style={{
        position: 'relative', zIndex: 5,
        flex: 1, display: 'flex', flexDirection: 'column',
        padding: '20px 32px 0',
        minHeight: 0,
      }}>
        <div style={{ flex: 1, display: 'flex', gap: 32, minHeight: 0 }}>

          {/* LEFT — Title + Years (Scene Selection) */}
          <div style={{ width: 540, flexShrink: 0, display: 'flex', flexDirection: 'column' }}>
            {/* The big chrome title */}
            <div style={{ position: 'relative', alignSelf: 'flex-start' }}>
              <div style={{
                fontSize: 12, fontWeight: 700, letterSpacing: 4, textTransform: 'uppercase',
                color: '#3ac8ff', textShadow: '0 0 10px rgba(58,200,255,0.7)',
                marginBottom: 4,
              }}>▸ Main Menu</div>
              <div style={{
                fontFamily: '"Helvetica Neue", Arial, sans-serif',
                fontSize: 56, fontWeight: 900,
                lineHeight: 0.92, letterSpacing: -2.5,
                background: 'linear-gradient(180deg, #fff 0%, #ff3a8c 50%, #c83aff 100%)',
                WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent',
                backgroundClip: 'text',
                filter: 'drop-shadow(0 2px 0 rgba(0,0,0,0.4)) drop-shadow(0 0 24px rgba(255,58,140,0.4))',
              }}>FAMILY<br/>ROOM</div>
              {/* subscript tagline */}
              <div style={{
                fontFamily: '"Courier New", monospace',
                fontSize: 10, fontWeight: 700, letterSpacing: 2,
                color: 'rgba(220,210,255,0.5)', textTransform: 'uppercase',
                marginTop: 8,
              }}>4,218 memories · 8 in the room · est. 2019</div>
            </div>

            {/* PLAY / SCENE SELECTION buttons */}
            <div style={{ display: 'flex', gap: 10, marginTop: 22 }}>
              <ChromeTile accent="#3aff9c" selected style={{ padding: '11px 18px', display: 'flex', alignItems: 'center', gap: 9 }}>
                <div style={{
                  width: 0, height: 0,
                  borderLeft: '10px solid #3aff9c',
                  borderTop: '7px solid transparent',
                  borderBottom: '7px solid transparent',
                  filter: 'drop-shadow(0 0 4px #3aff9c)',
                }} />
                <div style={{
                  fontSize: 11, fontWeight: 800, letterSpacing: 2.4, textTransform: 'uppercase',
                  color: '#fff', textShadow: '0 0 8px #3aff9c',
                }}>Play All</div>
              </ChromeTile>
              <ChromeTile accent="#3ac8ff" style={{ padding: '11px 14px' }}>
                <div style={{
                  fontSize: 11, fontWeight: 700, letterSpacing: 2, textTransform: 'uppercase',
                  color: 'rgba(220,230,255,0.85)',
                }}>On This Day</div>
              </ChromeTile>
              <ChromeTile accent="#ffcc28" style={{ padding: '11px 14px' }}>
                <div style={{
                  fontSize: 11, fontWeight: 700, letterSpacing: 2, textTransform: 'uppercase',
                  color: 'rgba(220,230,255,0.85)',
                }}>Setup</div>
              </ChromeTile>
            </div>

            {/* Years — Scene Selection grid */}
            <div style={{ marginTop: 32 }}>
              <div style={{
                fontSize: 11, fontWeight: 800, letterSpacing: 3.2, textTransform: 'uppercase',
                color: '#ff3a8c', textShadow: '0 0 8px rgba(255,58,140,0.55)',
                marginBottom: 14,
                display: 'flex', alignItems: 'center', gap: 10,
              }}>
                <div style={{ width: 8, height: 8, background: '#ff3a8c', boxShadow: '0 0 8px #ff3a8c' }} />
                Scene Selection · Years
                <div style={{ flex: 1, height: 0.5, background: 'linear-gradient(90deg, rgba(255,58,140,0.5), transparent)' }} />
              </div>

              <div style={{ display: 'flex', gap: 12 }}>
                {DVD_YEARS.map((y) => (
                  <DvdYearTile key={y.y} year={y} selected={y.y === selectedYear} />
                ))}
              </div>
            </div>

            {/* Marquee */}
            <div style={{ flex: 1 }} />
            <div style={{ marginBottom: 18, marginTop: 18 }}>
              <DvdMarquee text="◂ ◂ ◂ ON THIS DAY · THREE APRILS AGO · HENRY CAME HOME · 18 PHOTOS · 2 VIDEOS · QUIET SPRING AFTERNOON ▸ ▸ ▸" />
            </div>
          </div>

          {/* RIGHT — Library */}
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
            <div style={{
              display: 'flex', alignItems: 'center', justifyContent: 'space-between',
              marginBottom: 14,
            }}>
              <div>
                <div style={{
                  fontSize: 11, fontWeight: 800, letterSpacing: 3.2, textTransform: 'uppercase',
                  color: '#c83aff', textShadow: '0 0 8px rgba(200,58,255,0.55)',
                  marginBottom: 2,
                }}>Bonus Features</div>
                <div style={{
                  fontFamily: '"Helvetica Neue", Arial, sans-serif',
                  fontSize: 30, fontWeight: 900, color: '#fff',
                  letterSpacing: -0.8, textShadow: '0 0 20px rgba(200,58,255,0.4), 0 2px 0 rgba(0,0,0,0.4)',
                }}>The Library</div>
              </div>
              <div style={{ display: 'flex', gap: 6 }}>
                {['Recent', 'Loved', 'All', 'Auto'].map((l, i) => (
                  <div key={l} style={{
                    padding: '5px 10px', borderRadius: 3,
                    fontSize: 9.5, fontWeight: 800, letterSpacing: 1.6, textTransform: 'uppercase',
                    background: i === 0 ? 'linear-gradient(180deg, #c83aff 0%, #8a18c0 100%)' : 'rgba(255,255,255,0.06)',
                    color: i === 0 ? '#fff' : 'rgba(220,210,255,0.65)',
                    boxShadow: i === 0
                      ? '0 0 12px rgba(200,58,255,0.6), inset 0 1px 0 rgba(255,255,255,0.4)'
                      : 'inset 0 0 0 0.5px rgba(140,100,200,0.4)',
                    textShadow: i === 0 ? '0 1px 2px rgba(0,0,0,0.5)' : 'none',
                  }}>{l}</div>
                ))}
              </div>
            </div>

            {/* Library grid — 4 rows × 3 cols */}
            <div style={{
              flex: 1,
              display: 'grid',
              gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
              gridAutoRows: 'minmax(0, 1fr)',
              gap: 10,
              minHeight: 0, minWidth: 0,
            }}>
              {[
                { hue: '#ff3a8c', label: 'Lake House',     sub: 'Aug 25' },
                { hue: '#3aff9c', label: "Spring Walks",   sub: 'Apr 25' },
                { hue: '#ffcc28', label: "Garden",         sub: 'Jul 25' },
                { hue: '#3ac8ff', label: "Margot's Day",   sub: 'May 25' },
                { hue: '#c83aff', label: "Iris Rides",     sub: 'Jun 25', selected: true },
                { hue: '#ff3a8c', label: "Cabin Trip",     sub: 'Sep 25' },
                { hue: '#3aff9c', label: "Sunday Picnic",  sub: 'Aug 25' },
                { hue: '#ffcc28', label: "Henry @ 72",     sub: 'Aug 25' },
                { hue: '#3ac8ff', label: "Birthday",       sub: 'Mar 26' },
                { hue: '#c83aff', label: "Morning Walks",  sub: 'Feb 26' },
                { hue: '#ff3a8c', label: "Sunday Dinner",  sub: 'Mar 26' },
                { hue: '#3aff9c', label: "Cora's Studio",  sub: 'Apr 26' },
              ].map((tile, i) => (
                <DvdLibraryTile key={i} hue={tile.hue} label={tile.label} sub={tile.sub} selected={tile.selected} />
              ))}
            </div>
          </div>
        </div>

        {/* BOTTOM RAIL — Family chapter selector */}
        <div style={{ flexShrink: 0, paddingBottom: 14, paddingTop: 16 }}>
          <div style={{
            position: 'relative',
            background: 'linear-gradient(180deg, rgba(255,255,255,0.04) 0%, rgba(10,4,30,0.6) 100%)',
            borderRadius: 6,
            padding: '14px 32px 14px',
            boxShadow:
              'inset 0 1px 0 rgba(180,140,255,0.25), ' +
              'inset 0 -1px 0 rgba(0,0,0,0.5), ' +
              '0 0 0 0.5px rgba(140,100,200,0.4)',
          }}>
            {/* label strip */}
            <div style={{
              position: 'absolute', top: -7, left: 18,
              padding: '2px 8px',
              fontSize: 9.5, fontWeight: 800, letterSpacing: 2.4, textTransform: 'uppercase',
              color: '#3aff9c', textShadow: '0 0 6px #3aff9c',
              background: '#0a0420',
            }}>Audio · Cast</div>

            {/* connector wire */}
            <div style={{
              position: 'absolute',
              left: 70, right: 70, top: '50%',
              height: 2, transform: 'translateY(-1px)',
              background: 'linear-gradient(90deg, rgba(255,58,140,0.3) 0%, rgba(58,255,156,0.3) 25%, rgba(58,200,255,0.3) 50%, rgba(255,204,40,0.3) 75%, rgba(200,58,255,0.3) 100%)',
              borderRadius: 1,
              boxShadow: '0 0 6px rgba(255,255,255,0.15)',
            }} />

            <div style={{
              display: 'flex', justifyContent: 'space-between', alignItems: 'center',
              position: 'relative',
            }}>
              {DVD_PEOPLE.map((p) => (
                <DvdPersonDot key={p.n} p={p} selected={p.n === selectedPerson} />
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

window.MacRoomFrame = MacRoomFrame;
window.MacRoomApp = MacRoomApp;
