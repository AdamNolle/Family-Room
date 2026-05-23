// Family Room — detail view, labeling, upload, family, memory screens

// ═══════════════════════════════════════════════════════════════
// SCREEN 2 — PHOTO / VIDEO DETAIL
// Full-bleed image on linen mat with labels as fabric tags.
// ═══════════════════════════════════════════════════════════════
function DetailScreen({ warmth = 'afternoon', intensity = 1 }) {
  const labels = [
    { l: 'Iris',     type: 'person', hue: 340 },
    { l: 'Theo',     type: 'person', hue: 30 },
    { l: 'back road', type: 'place' },
    { l: 'first ride', type: 'event' },
    { l: 'summer 2025', type: 'time' },
  ];
  return (
    <ScreenShell warmth={warmth} intensity={intensity} bg="#efe1c6">
      {/* top bar */}
      <div style={{ position: 'absolute', top: 58, left: 16, right: 16, zIndex: 10, display: 'flex', justifyContent: 'space-between' }}>
        <GlassBtn icon={Icon.chevronL} />
        <div style={{ display: 'flex', gap: 8 }}>
          <GlassBtn icon={Icon.heart} />
          <GlassBtn icon={Icon.share} />
          <GlassBtn icon={Icon.dots} />
        </div>
      </div>

      {/* image hero — on a linen mat */}
      <div style={{ padding: '112px 20px 0' }}>
        <LinenSurface tone="cream" radius={28} style={{
          padding: 12, boxShadow: '0 20px 40px -15px rgba(80,40,10,0.35), 0 0 0 0.5px rgba(140,90,50,0.12)',
        }}>
          <div style={{ borderRadius: 18, overflow: 'hidden', position: 'relative' }}>
            <PhotoPlaceholder label="IRIS LEARNS TO RIDE · 00:42" hue={180} ratio={3/4} />
            {/* play button overlay */}
            <div style={{
              position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center',
            }}>
              <div style={{
                width: 64, height: 64, borderRadius: 32,
                background: 'rgba(255,245,220,0.35)', backdropFilter: 'blur(20px)',
                border: '0.5px solid rgba(255,255,255,0.5)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                paddingLeft: 4,
              }}>
                {Icon.play(28, '#fff')}
              </div>
            </div>
            {/* scrubber */}
            <div style={{
              position: 'absolute', bottom: 10, left: 10, right: 10, height: 4,
              background: 'rgba(255,255,255,0.25)', borderRadius: 2, backdropFilter: 'blur(10px)',
            }}>
              <div style={{ width: '28%', height: '100%', background: 'rgba(255,245,220,0.95)', borderRadius: 2 }} />
            </div>
          </div>
        </LinenSurface>
      </div>

      {/* meta */}
      <div style={{ padding: '22px 28px 0' }}>
        <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#8a6540', letterSpacing: 1.2, textTransform: 'uppercase' }}>
          Saturday · June 14
        </div>
        <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 28, color: '#2a1a10', marginTop: 4, lineHeight: 1.15 }}>
          Iris learns to ride
        </div>
        <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#6a4a28', marginTop: 6, lineHeight: 1.5 }}>
          Filmed by Theo, on the back road behind the lake house. She got three pedals in before the wobble.
        </div>
      </div>

      {/* labels as fabric tags */}
      <div style={{ padding: '22px 22px 0' }}>
        <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 16, color: '#2a1a10', marginBottom: 10, letterSpacing: 0.2 }}>
          Tags
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
          {labels.map((L, i) => <FabricTag key={i} {...L} />)}
          <div style={{
            padding: '7px 14px', borderRadius: 999,
            border: '1px dashed rgba(140,90,50,0.4)', color: '#8a6540',
            fontFamily: 'system-ui', fontSize: 13, display: 'flex', alignItems: 'center', gap: 6,
          }}>{Icon.plus(14, '#8a6540')} Add tag</div>
        </div>
      </div>

      {/* saved by */}
      <div style={{ padding: '22px 22px 0' }}>
        <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 16, color: '#2a1a10', marginBottom: 10 }}>
          Loved by
        </div>
        <div style={{ display: 'flex', gap: -6, alignItems: 'center' }}>
          {['Margot','Henry','Clara','Lena'].map((n, i) => (
            <div key={i} style={{ marginLeft: i === 0 ? 0 : -10 }}>
              <Avatar name={n} hue={MEMBERS.find(m=>m.n===n)?.hue || 30} size={30} ring />
            </div>
          ))}
          <div style={{ fontFamily: 'system-ui', fontSize: 13, color: '#6a4a28', marginLeft: 10 }}>
            and 2 others
          </div>
        </div>
      </div>

      {/* note strip */}
      <div style={{ padding: '22px 22px 80px' }}>
        <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 16, color: '#2a1a10', marginBottom: 10 }}>
          Notes from the family
        </div>
        <NoteStrip name="Margot" hue={20} text="Oh my heart. She looks so determined." />
        <NoteStrip name="Henry" hue={35} text="That's the same road I taught her mother on." />
      </div>
    </ScreenShell>
  );
}

function GlassBtn({ icon }) {
  return (
    <div style={{
      width: 38, height: 38, borderRadius: 19,
      background: 'rgba(255,245,220,0.55)', backdropFilter: 'blur(14px) saturate(180%)',
      border: '0.5px solid rgba(140,90,50,0.18)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      boxShadow: '0 2px 8px rgba(80,40,10,0.1)',
    }}>{icon(18, '#5a2a0a')}</div>
  );
}

function FabricTag({ l, type, hue = 30 }) {
  const colors = {
    person: { bg: `hsl(${hue},35%,82%)`, fg: `hsl(${hue},50%,28%)` },
    place:  { bg: '#d8d0b8', fg: '#4a4028' },
    event:  { bg: '#e6c8a0', fg: '#5a3818' },
    time:   { bg: '#d4c4a8', fg: '#4a3820' },
  };
  const c = colors[type] || colors.event;
  return (
    <div style={{
      padding: '6px 12px 6px 8px', borderRadius: 999,
      background: c.bg, color: c.fg,
      fontFamily: 'system-ui', fontSize: 13, fontWeight: 500,
      display: 'flex', alignItems: 'center', gap: 6,
      boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.4), 0 1px 2px rgba(80,40,10,0.08)',
      position: 'relative',
    }}>
      <span style={{ width: 16, height: 16, borderRadius: 8, background: `hsl(${hue||30},45%,55%)`, display: 'inline-block' }} />
      {l}
    </div>
  );
}

function NoteStrip({ name, hue, text }) {
  return (
    <div style={{ display: 'flex', gap: 10, marginBottom: 14 }}>
      <Avatar name={name} hue={hue} size={32} />
      <div style={{ flex: 1, background: '#fff8ea', borderRadius: 16, padding: '10px 14px',
        boxShadow: '0 1px 2px rgba(80,40,10,0.06), 0 0 0 0.5px rgba(140,90,50,0.1)' }}>
        <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#8a6540', fontWeight: 600 }}>{name}</div>
        <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 15, color: '#2a1a10', marginTop: 2, lineHeight: 1.4 }}>{text}</div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// SCREEN 3 — LABEL / TAG FLOW
// ═══════════════════════════════════════════════════════════════
function LabelScreen({ warmth = 'afternoon', intensity = 1 }) {
  return (
    <ScreenShell warmth={warmth} intensity={intensity} bg="#f2e4ca">
      <HeaderPad>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 20px' }}>
          <div style={{ fontFamily: 'system-ui', fontSize: 15, color: '#8a6540' }}>Cancel</div>
          <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#5a2a0a', fontWeight: 600 }}>Label · 3 of 24</div>
          <div style={{ fontFamily: 'system-ui', fontSize: 15, color: '#5a2a0a', fontWeight: 600 }}>Done</div>
        </div>

        {/* photo with draggable tag pins */}
        <div style={{ padding: '16px 20px 0' }}>
          <LinenSurface tone="oat" radius={24} style={{ padding: 10 }}>
            <div style={{ borderRadius: 16, overflow: 'hidden', position: 'relative' }}>
              <PhotoPlaceholder label="SUNDAY LUNCH · KITCHEN" hue={28} ratio={1} />
              {/* pin — person */}
              <TagPin x="28%" y="34%" name="Margot" hue={20} />
              <TagPin x="60%" y="44%" name="Clara" hue={12} />
              <TagPin x="48%" y="68%" name="Iris" hue={340} />
            </div>
          </LinenSurface>
        </div>

        {/* suggested */}
        <div style={{ padding: '22px 22px 0' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 10 }}>
            {Icon.sparkle(14, '#b08040')}
            <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 15, color: '#2a1a10', letterSpacing: 0.2 }}>
              Suggested — tap to add
            </div>
          </div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            <FabricTag l="Henry" type="person" hue={35} />
            <FabricTag l="kitchen" type="place" />
            <FabricTag l="Sunday lunch" type="event" />
            <FabricTag l="June 2025" type="time" />
            <FabricTag l="Theo" type="person" hue={30} />
          </div>
        </div>

        {/* current */}
        <div style={{ padding: '22px 22px 0' }}>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 15, color: '#2a1a10', marginBottom: 10 }}>
            Already labeled
          </div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            <FabricTag l="Margot" type="person" hue={20} />
            <FabricTag l="Clara" type="person" hue={12} />
            <FabricTag l="Iris" type="person" hue={340} />
          </div>
        </div>

        {/* quick add field — looks like a linen-bound journal entry */}
        <div style={{ padding: '28px 20px 120px' }}>
          <LinenSurface tone="cream" radius={18} style={{ padding: '14px 16px' }}>
            <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#8a6540', letterSpacing: 1, textTransform: 'uppercase' }}>
              Add a memory
            </div>
            <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 18, color: '#6a4a28', marginTop: 4, fontStyle: 'italic' }}>
              The last Sunday before the move...
            </div>
            <div style={{ height: 1, background: 'rgba(140,90,50,0.2)', marginTop: 10 }} />
            <div style={{ height: 1, background: 'rgba(140,90,50,0.2)', marginTop: 16 }} />
            <div style={{ height: 1, background: 'rgba(140,90,50,0.2)', marginTop: 16 }} />
          </LinenSurface>
        </div>
      </HeaderPad>

      {/* floating suggestion footer */}
      <div style={{ position: 'absolute', bottom: 24, left: 20, right: 20, zIndex: 10 }}>
        <div style={{
          padding: '14px 16px', borderRadius: 22,
          background: 'rgba(70,45,25,0.92)', backdropFilter: 'blur(16px)',
          color: '#fff4d6', display: 'flex', alignItems: 'center', gap: 12,
          boxShadow: '0 12px 30px rgba(40,20,10,0.35)',
        }}>
          {Icon.sparkle(18, '#ffe0a8')}
          <div style={{ flex: 1, fontFamily: 'system-ui', fontSize: 13, lineHeight: 1.35 }}>
            Looks like <b>Sunday lunch</b> — label all 24 photos at once?
          </div>
          <div style={{
            padding: '6px 12px', borderRadius: 14, background: '#e8a95a', color: '#2a1a10',
            fontFamily: 'system-ui', fontSize: 13, fontWeight: 600,
          }}>Yes</div>
        </div>
      </div>
    </ScreenShell>
  );
}

function TagPin({ x, y, name, hue }) {
  return (
    <div style={{ position: 'absolute', left: x, top: y, transform: 'translate(-50%, -50%)', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2 }}>
      <div style={{
        padding: '3px 10px 3px 3px', borderRadius: 999, background: 'rgba(40,20,10,0.75)',
        backdropFilter: 'blur(10px)', display: 'flex', alignItems: 'center', gap: 6,
        color: '#fff4d6', fontFamily: 'system-ui', fontSize: 11, fontWeight: 500,
        boxShadow: '0 4px 10px rgba(0,0,0,0.3)',
      }}>
        <Avatar name={name} hue={hue} size={20} />
        {name}
      </div>
      <div style={{ width: 1, height: 10, background: 'rgba(255,255,255,0.75)' }} />
      <div style={{ width: 10, height: 10, borderRadius: 5, background: '#fff4d6', boxShadow: `0 0 0 2px hsl(${hue},50%,55%), 0 2px 6px rgba(0,0,0,0.4)` }} />
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// SCREEN 4 — UPLOAD / CONTRIBUTE
// ═══════════════════════════════════════════════════════════════
function UploadScreen({ warmth = 'afternoon', intensity = 1 }) {
  const queue = [
    { name: 'IMG_4821.HEIC', size: '3.4 MB',   status: 'done', hue: 28 },
    { name: 'IMG_4822.HEIC', size: '3.1 MB',   status: 'done', hue: 40 },
    { name: 'IMG_4823.HEIC', size: '4.0 MB',   status: 'up',   hue: 180, pct: 62 },
    { name: 'IMG_4824.MOV',  size: '124 MB',   status: 'queue', hue: 200 },
    { name: 'IMG_4825.HEIC', size: '3.8 MB',   status: 'queue', hue: 12 },
  ];
  return (
    <ScreenShell warmth={warmth} intensity={intensity} bg="#f5e8ce">
      <HeaderPad>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 20px' }}>
          <div style={{ color: '#8a6540' }}>{Icon.close(22, '#8a6540')}</div>
          <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#5a2a0a', fontWeight: 600 }}>Bring in photos</div>
          <div style={{ width: 22 }} />
        </div>

        <div style={{ padding: '18px 20px 0' }}>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 32, color: '#2a1a10', lineHeight: 1.1, letterSpacing: -0.3 }}>
            Share with<br/>The Alderhouse
          </div>
          <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#7a5a3a', marginTop: 8 }}>
            5 items selected · Clara will be credited
          </div>
        </div>

        {/* destination picker */}
        <div style={{ padding: '22px 20px 0' }}>
          <WoodSurface tone="oak" radius={20} style={{ padding: '14px 18px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <div style={{ width: 44, height: 44, borderRadius: 10, background: 'rgba(255,245,220,0.9)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {Icon.book(22, '#6a3a10')}
              </div>
              <div style={{ flex: 1 }}>
                <div style={{ fontFamily: 'system-ui', fontSize: 11, color: 'rgba(255,244,214,0.7)', letterSpacing: 1, textTransform: 'uppercase' }}>Album</div>
                <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 18, color: '#fff4d6' }}>Sunday lunches · 2025</div>
              </div>
              {Icon.chevronR(18, 'rgba(255,244,214,0.75)')}
            </div>
          </WoodSurface>
        </div>

        {/* tag row */}
        <div style={{ padding: '16px 22px 0' }}>
          <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#8a6540', letterSpacing: 1, textTransform: 'uppercase', marginBottom: 8 }}>
            Apply to all
          </div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            <FabricTag l="Margot" type="person" hue={20} />
            <FabricTag l="kitchen" type="place" />
            <FabricTag l="Sunday lunch" type="event" />
          </div>
        </div>

        {/* queue */}
        <div style={{ padding: '22px 22px 120px' }}>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 16, color: '#2a1a10', marginBottom: 12 }}>
            Queue
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {queue.map((q, i) => <QueueRow key={i} {...q} />)}
          </div>
        </div>
      </HeaderPad>

      {/* Submit */}
      <div style={{ position: 'absolute', bottom: 24, left: 20, right: 20, zIndex: 10 }}>
        <div style={{
          height: 54, borderRadius: 27, display: 'flex', alignItems: 'center', justifyContent: 'center',
          background: 'linear-gradient(160deg, #e8a95a, #b36a2a)',
          color: '#fff', fontFamily: 'system-ui', fontSize: 16, fontWeight: 600, letterSpacing: 0.2,
          boxShadow: '0 12px 30px rgba(140,60,10,0.35), inset 0 1px 0 rgba(255,230,180,0.5)',
          gap: 8,
        }}>
          {Icon.upload(18, '#fff')} Share 5 items
        </div>
      </div>
    </ScreenShell>
  );
}

function QueueRow({ name, size, status, hue, pct }) {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 12, padding: 10, borderRadius: 16,
      background: 'rgba(255,248,234,0.7)',
      boxShadow: '0 0 0 0.5px rgba(140,90,50,0.1)',
    }}>
      <div style={{ width: 44, height: 44, borderRadius: 10, overflow: 'hidden', flexShrink: 0 }}>
        <PhotoPlaceholder hue={hue} ratio={1} />
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontFamily: 'ui-monospace, monospace', fontSize: 12, color: '#2a1a10' }}>{name}</div>
        <div style={{ fontFamily: 'system-ui', fontSize: 11, color: '#8a6540', marginTop: 2 }}>
          {status === 'done' ? `${size} · shared` : status === 'up' ? `${size} · ${pct}%` : `${size} · waiting`}
        </div>
        {status === 'up' && (
          <div style={{ height: 3, borderRadius: 2, background: 'rgba(140,90,50,0.15)', marginTop: 5, overflow: 'hidden' }}>
            <div style={{ height: '100%', width: `${pct}%`, background: 'linear-gradient(90deg, #e8a95a, #b36a2a)' }} />
          </div>
        )}
      </div>
      {status === 'done' && <div style={{ width: 22, height: 22, borderRadius: 11, background: '#5a8a3a', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>{Icon.check(14, '#fff')}</div>}
      {status === 'queue' && <div style={{ color: '#b08860' }}>{Icon.dots(22, '#b08860')}</div>}
    </div>
  );
}

window.DetailScreen = DetailScreen;
window.LabelScreen = LabelScreen;
window.UploadScreen = UploadScreen;
window.FabricTag = FabricTag;
