// Albums + Video Composer screens

const ALBUMS = [
  { t: "Henry's 72nd",       sub: '61 photos · 4 videos', cover: 'mustard', contributors: 4, pinned: true,  auto: false },
  { t: "Iris's first year",  sub: '204 photos · 12 videos', cover: 'rose', contributors: 3, pinned: true,  auto: true },
  { t: "Sunday lunches",     sub: 'Ongoing · 124 photos',  cover: 'butter', contributors: 6, pinned: false, auto: true },
  { t: "Summer 2025",        sub: '318 photos · 22 videos', cover: 'clay', contributors: 7, pinned: false, auto: false },
  { t: "The cabin",          sub: '142 photos · 8 videos',  cover: 'terra', contributors: 5, pinned: false, auto: false },
  { t: "Garden through the years", sub: '82 photos · 3 videos', cover: 'moss', contributors: 2, pinned: false, auto: true },
];

// ═══════════════════════════════════════════════════════════════
// iPhone ALBUMS
// ═══════════════════════════════════════════════════════════════
function AlbumsV2({ mode = 'natural', intensity = 1 }) {
  return (
    <Shell mode={mode} intensity={intensity}>
      <div style={{ paddingTop: 58 }}>
        <div style={{ padding: '0 20px', display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
          <div>
            <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.8, color: INK.mute, textTransform: 'uppercase' }}>Collected</div>
            <div style={{ fontFamily: R_FONT, fontSize: 40, color: INK.ink, fontWeight: 700, marginTop: 4, lineHeight: 1, letterSpacing: -0.8 }}>Albums</div>
          </div>
          <SoftButton tone="claret" icon={Icon.plus(13, '#f5e0c8')}>New</SoftButton>
        </div>

        <div style={{ padding: '14px 20px 0', display: 'flex', gap: 7, overflowX: 'auto' }}>
          {['All','Pinned','Auto-made','Shared','Drafts'].map((l, i) => (
            <SoftButton key={i} tone={i === 0 ? 'butter' : 'ghost'}>{l}</SoftButton>
          ))}
        </div>

        {/* pinned */}
        <div style={{ padding: '20px 20px 0' }}>
          <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.4, color: INK.mute, textTransform: 'uppercase', marginBottom: 10 }}>Pinned to the room</div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
            {ALBUMS.filter(a => a.pinned).map((a, i) => (
              <SoftFabric key={i} tone={a.cover} radius={18} style={{ padding: 6 }} pressable>
                <div style={{ position: 'relative', borderRadius: 13, overflow: 'hidden' }}>
                  <Photo tone={a.cover} ratio={1} mode={mode} />
                  {a.auto && (
                    <div style={{ position: 'absolute', top: 7, left: 7, padding: '3px 7px', borderRadius: 999, background: 'rgba(255,248,220,0.85)', fontFamily: R_FONT, fontSize: 9, fontWeight: 700, letterSpacing: 1, textTransform: 'uppercase', color: '#4a2610' }}>
                      Auto
                    </div>
                  )}
                </div>
                <div style={{ padding: '9px 4px 2px' }}>
                  <div style={{ fontFamily: R_FONT, fontSize: 13, fontWeight: 700, lineHeight: 1.15 }}>{a.t}</div>
                  <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 500, opacity: 0.7, marginTop: 2 }}>{a.sub}</div>
                </div>
              </SoftFabric>
            ))}
          </div>
        </div>

        {/* all */}
        <div style={{ padding: '24px 20px 0' }}>
          <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.4, color: INK.mute, textTransform: 'uppercase', marginBottom: 10 }}>All albums</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {ALBUMS.slice(2).map((a, i) => (
              <SoftFabric key={i} tone="paper" radius={16} style={{ padding: 8, display: 'flex', gap: 10, alignItems: 'center' }} pressable>
                <div style={{ width: 56, height: 56, borderRadius: 11, overflow: 'hidden', flexShrink: 0, boxShadow: 'inset 0 0 0 0.5px rgba(40,15,10,0.14)' }}>
                  <Photo tone={a.cover} ratio={1} mode={mode} />
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontFamily: R_FONT, fontSize: 14, fontWeight: 700, color: INK.ink }}>{a.t}</div>
                  <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 500, color: INK.soft, marginTop: 2 }}>{a.sub}</div>
                </div>
                <div style={{ display: 'flex' }}>
                  {MEMBERS.slice(0, Math.min(3, a.contributors)).map((m, j) => (
                    <div key={j} style={{ marginLeft: j === 0 ? 0 : -8 }}>
                      <AvatarR name={m.n} tone={m.tone} size={22} />
                    </div>
                  ))}
                </div>
              </SoftFabric>
            ))}
          </div>
        </div>

        {/* create a reel — entry point to composer */}
        <div style={{ padding: '24px 20px 130px' }}>
          <SoftFabric tone="claret" radius={22} style={{ padding: 18 }} pressable>
            <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
              <div style={{
                width: 54, height: 54, borderRadius: 15,
                background: 'linear-gradient(180deg, #f0a890, #b0492e)',
                boxShadow: 'inset 0 1.5px 0 rgba(255,230,200,0.5), 0 6px 14px -4px rgba(140,60,35,0.5)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
              }}>{Icon.play(24, '#fff4d6')}</div>
              <div style={{ flex: 1 }}>
                <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 700, color: 'rgba(246,224,200,0.72)', letterSpacing: 1.4, textTransform: 'uppercase' }}>Make a film</div>
                <div style={{ fontFamily: R_FONT, fontSize: 17, fontWeight: 700, color: '#f5e0c8', marginTop: 2 }}>Stitch videos with chapters</div>
                <div style={{ fontFamily: R_FONT, fontSize: 11.5, fontWeight: 500, color: 'rgba(246,224,200,0.7)', marginTop: 2 }}>On-device · no cloud</div>
              </div>
              {Icon.chevronR(18, 'rgba(246,224,200,0.7)')}
            </div>
          </SoftFabric>
        </div>
      </div>
    </Shell>
  );
}

// ═══════════════════════════════════════════════════════════════
// iPhone VIDEO COMPOSER — timeline with chapters
// ═══════════════════════════════════════════════════════════════
const CHAPTERS = [
  { t: 'Waking up',       len: 12, clips: 3, tone: 'butter' },
  { t: 'At the lake',     len: 48, clips: 8, tone: 'sky' },
  { t: 'Birthday candles', len: 22, clips: 4, tone: 'mustard' },
  { t: 'Fireworks',       len: 18, clips: 5, tone: 'claret' },
  { t: 'Quiet drive home', len: 14, clips: 2, tone: 'lavender' },
];

function ComposerV2({ mode = 'natural', intensity = 1 }) {
  const totalSec = CHAPTERS.reduce((s, c) => s + c.len, 0);
  return (
    <Shell mode={mode} intensity={intensity}>
      <div style={{ paddingTop: 58 }}>
        <div style={{ padding: '0 20px', display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
          <div>
            <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.8, color: INK.mute, textTransform: 'uppercase' }}>Compose · draft</div>
            <div style={{ fontFamily: R_FONT, fontSize: 30, color: INK.ink, fontWeight: 700, marginTop: 4, lineHeight: 1.05, letterSpacing: -0.6 }}>Summer at the lake</div>
            <div style={{ fontFamily: R_FONT, fontSize: 12, fontWeight: 500, color: INK.soft, marginTop: 5 }}>{CHAPTERS.length} chapters · {Math.floor(totalSec / 60)}m {totalSec % 60}s · 1080p</div>
          </div>
          <SoftButton tone="claret" icon={Icon.upload(13, '#f5e0c8')}>Export</SoftButton>
        </div>

        {/* preview — polaroid-ish with play chrome */}
        <div style={{ padding: '22px 20px 0' }}>
          <SoftFabric tone="paper" radius={18} style={{ padding: 6 }}>
            <div style={{ position: 'relative', borderRadius: 13, overflow: 'hidden' }}>
              <Photo tone="sky" label="AT THE LAKE · CHAPTER 2" ratio={16/9} mode={mode} />
              <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <div style={{
                  width: 64, height: 64, borderRadius: 32,
                  background: 'rgba(255,248,220,0.85)',
                  backdropFilter: 'blur(8px)',
                  boxShadow: 'inset 0 1.5px 0 rgba(255,255,255,0.6), 0 8px 20px -4px rgba(40,15,10,0.5)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>{Icon.play(26, '#3a1810')}</div>
              </div>
              {/* scrubber */}
              <div style={{ position: 'absolute', left: 10, right: 10, bottom: 10, height: 3, borderRadius: 2, background: 'rgba(255,248,220,0.3)' }}>
                <div style={{ position: 'absolute', left: 0, top: 0, bottom: 0, width: '38%', background: 'rgba(255,248,220,0.9)', borderRadius: 2 }} />
                <div style={{ position: 'absolute', left: '38%', top: -3, width: 9, height: 9, borderRadius: 5, background: '#fff8e0', boxShadow: '0 1px 3px rgba(40,15,10,0.4)', transform: 'translateX(-50%)' }} />
              </div>
              {/* chapter ticks */}
              {(() => {
                let acc = 0;
                return CHAPTERS.map((c, i) => {
                  acc += c.len;
                  const x = (acc / totalSec) * 100;
                  if (i === CHAPTERS.length - 1) return null;
                  return (
                    <div key={i} style={{ position: 'absolute', left: `${x}%`, bottom: 8, width: 1, height: 7, background: 'rgba(255,248,220,0.85)' }} />
                  );
                });
              })()}
            </div>
          </SoftFabric>
        </div>

        {/* chapter list */}
        <div style={{ padding: '22px 20px 0' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
            <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.4, color: INK.mute, textTransform: 'uppercase' }}>Chapters</div>
            <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 600, color: INK.soft }}>Drag to reorder</div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {CHAPTERS.map((c, i) => (
              <SoftFabric key={i} tone={i === 1 ? c.tone : 'paper'} radius={14} style={{ padding: 8, display: 'flex', gap: 10, alignItems: 'center' }} pressable>
                <div style={{ fontFamily: R_FONT, fontSize: 12, fontWeight: 700, color: i === 1 ? 'rgba(58,24,16,0.55)' : INK.mute, width: 16, textAlign: 'center' }}>{i + 1}</div>
                <div style={{ width: 52, height: 52, borderRadius: 10, overflow: 'hidden', flexShrink: 0, boxShadow: 'inset 0 0 0 0.5px rgba(40,15,10,0.14)' }}>
                  <Photo tone={c.tone} ratio={1} mode={mode} />
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontFamily: R_FONT, fontSize: 14, fontWeight: 700, color: i === 1 ? '#3a1810' : INK.ink }}>{c.t}</div>
                  <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 500, opacity: 0.7, marginTop: 2 }}>
                    {c.clips} clips · {Math.floor(c.len / 60) ? `${Math.floor(c.len/60)}m ` : ''}{c.len % 60}s
                  </div>
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 2, opacity: 0.45 }}>
                  <div style={{ width: 14, height: 1.5, background: 'currentColor', borderRadius: 1 }} />
                  <div style={{ width: 14, height: 1.5, background: 'currentColor', borderRadius: 1 }} />
                  <div style={{ width: 14, height: 1.5, background: 'currentColor', borderRadius: 1 }} />
                </div>
              </SoftFabric>
            ))}
            <SoftFabric tone="paper" radius={14} style={{ padding: '12px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8, opacity: 0.8 }} pressable>
              {Icon.plus(14, INK.soft)}
              <div style={{ fontFamily: R_FONT, fontSize: 13, fontWeight: 600, color: INK.soft }}>Add chapter</div>
            </SoftFabric>
          </div>
        </div>

        {/* suggestions */}
        <div style={{ padding: '22px 20px 140px' }}>
          <CozyGlass tint="butter" radius={18} style={{ padding: '14px 14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 10 }}>
              {Icon.sparkle(15, INK.ink)}
              <div style={{ fontFamily: R_FONT, fontSize: 13, fontWeight: 700, color: INK.ink }}>Family Room suggests</div>
            </div>
            <div style={{ fontFamily: R_FONT, fontSize: 12.5, fontWeight: 500, color: INK.soft, lineHeight: 1.5 }}>
              Add a title card with "Alderhouse · July 2025" · Use Margot's voice memo from Jul 14 as the opening line · Crossfade chapter 3 into chapter 4 — both have firelight
            </div>
          </CozyGlass>
        </div>
      </div>
    </Shell>
  );
}

Object.assign(window, { AlbumsV2, ComposerV2 });
