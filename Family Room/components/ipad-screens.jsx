// iPad — softer fabric canvas, cozy glass sidebar, springy rows.

function IPadFrame({ children, width = 1180, height = 820 }) {
  return (
    <div style={{
      width, height, borderRadius: 38, overflow: 'hidden', position: 'relative',
      background: '#d9c7a0',
      boxShadow: '0 40px 90px rgba(60,20,10,0.22), 0 0 0 1px rgba(0,0,0,0.1), 0 0 0 10px #1a0f08, 0 0 0 12px rgba(0,0,0,0.25)',
    }}>
      <div style={{ position: 'absolute', top: 10, left: '50%', transform: 'translateX(-50%)', width: 8, height: 8, borderRadius: 4, background: '#0a0604', zIndex: 100 }} />
      {children}
    </div>
  );
}

function IPadApp({ mode = 'natural', intensity = 1 }) {
  // Brighter ivory canvas — the amber was too heavy on a large surface.
  const base = mode === 'daylight' ? '#f5ddb4' : '#fcf2d9';
  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', background: base, overflow: 'hidden' }}>
      <WindowLight mode={mode} intensity={intensity} />
      <div style={{ position: 'relative', zIndex: 2, height: '100%', display: 'flex' }}>
        {/* Sidebar — cozy glass floating panel */}
        <div style={{ width: 270, padding: '16px 12px 16px 14px', display: 'flex', flexDirection: 'column', gap: 10, flexShrink: 0, overflow: 'auto' }}>
          <CozyGlass tint="butter" radius={20} style={{ padding: '14px 14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 11 }}>
              <div style={{ width: 40, height: 40, borderRadius: 12, overflow: 'hidden', background: 'radial-gradient(circle at 30% 25%, #b56464, #5a1a1a)', boxShadow: 'inset 0 1.5px 0 rgba(255,230,200,0.45)' }}>
                <div style={{ width: '100%', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#f4dfb2', fontFamily: R_FONT, fontWeight: 700, fontSize: 18 }}>A</div>
              </div>
              <div>
                <div style={{ fontFamily: R_FONT, fontSize: 16, fontWeight: 700, color: INK.ink, lineHeight: 1 }}>The Alderhouse</div>
                <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 500, color: INK.soft, marginTop: 3 }}>8 in the room · 4,218 memories</div>
              </div>
            </div>
          </CozyGlass>

          <CozyGlass tint="clear" radius={13} style={{ padding: '10px 12px', display: 'flex', alignItems: 'center', gap: 9 }}>
            {Icon.search(15, INK.mute)}
            <div style={{ fontFamily: R_FONT, fontSize: 13, fontWeight: 500, color: INK.mute }}>Search the room</div>
          </CozyGlass>

          <SideGroup title="Library">
            <SideRow icon={Icon.home} label="Everything" n="4,218" />
            <SideRow icon={Icon.calendar} label="On this day" n="18" />
            <SideRow icon={Icon.heartFill} label="Loved" n="312" />
          </SideGroup>
          <SideGroup title="Organize">
            <SideRow icon={Icon.calendar} label="Years" n="8" />
            <SideRow icon={Icon.people} label="People" n="8" on />
            <SideRow icon={Icon.sparkle} label="Events" n="142" />
            <SideRow icon={Icon.tag} label="Places" n="14" />
            <SideRow icon={Icon.book} label="Albums" n="26" />
          </SideGroup>
          <SideGroup title="Recent events">
            {EVENTS.slice(0, 3).map((e, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '7px 10px', borderRadius: 10 }}>
                <div style={{ width: 10, height: 10, borderRadius: 5, background: `var(--tone-${e.tone}, #c26a3e)`, boxShadow: 'inset 0 1px 0 rgba(255,240,210,0.4)' }} />
                <div style={{ flex: 1, fontFamily: R_FONT, fontSize: 12.5, fontWeight: 500, color: INK.soft, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.t}</div>
                <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 500, color: INK.mute }}>{e.count}</div>
              </div>
            ))}
          </SideGroup>

          <div style={{ marginTop: 'auto' }}>
            <SoftFabric tone="claret" radius={16} style={{ padding: 10 }}>
              <div style={{ fontFamily: R_FONT, fontSize: 10, fontWeight: 700, color: 'rgba(246,224,200,0.72)', letterSpacing: 1.3, textTransform: 'uppercase', marginBottom: 8, padding: '0 2px' }}>Family</div>
              <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                {MEMBERS.map((m, i) => <AvatarR key={i} name={m.n} tone={m.tone} size={30} />)}
              </div>
            </SoftFabric>
          </div>
        </div>

        {/* Main */}
        <div style={{ flex: 1, padding: '14px 18px 14px 4px', overflow: 'auto' }}>
          <SoftFabric tone="paper" radius={24} raised={false} style={{ padding: '28px 32px 32px', minHeight: '100%' }}>
            <IPadCanvas mode={mode} />
          </SoftFabric>
        </div>
      </div>
    </div>
  );
}

function SideGroup({ title, children }) {
  return (
    <div>
      <div style={{ padding: '14px 12px 4px', fontFamily: R_FONT, fontSize: 10.5, fontWeight: 700, letterSpacing: 1.4, color: INK.mute, textTransform: 'uppercase' }}>{title}</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>{children}</div>
    </div>
  );
}

function SideRow({ icon, label, n, on }) {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 10, padding: '8px 12px', borderRadius: 11,
      background: on ? 'linear-gradient(180deg, rgba(255,245,215,0.85), rgba(232,195,120,0.5))' : 'transparent',
      boxShadow: on ? 'inset 0 1px 0 rgba(255,245,220,0.5), 0 2px 6px -2px rgba(140,70,35,0.25)' : 'none',
      cursor: 'pointer',
    }}>
      {icon && icon(15, on ? INK.ink : INK.soft)}
      <div style={{ flex: 1, fontFamily: R_FONT, fontSize: 13, fontWeight: on ? 700 : 500, color: on ? INK.ink : INK.soft }}>{label}</div>
      <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 500, color: INK.mute }}>{n}</div>
    </div>
  );
}

function IPadCanvas({ mode }) {
  return (
    <>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
        <div>
          <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.8, color: INK.mute, textTransform: 'uppercase' }}>Everything · chronological</div>
          <div style={{ fontFamily: R_FONT, fontSize: 46, color: INK.ink, fontWeight: 700, lineHeight: 1, marginTop: 6, letterSpacing: -0.8 }}>The year, so far</div>
          <div style={{ fontFamily: R_FONT, fontSize: 13.5, fontWeight: 500, color: INK.soft, marginTop: 8 }}>218 memories in 2026 · 4 events · 8 contributors</div>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <SoftButton tone="ghost" icon={Icon.grid(13, INK.soft)}>Grid</SoftButton>
          <SoftButton tone="butter" icon={Icon.upload(13, INK.ink)}>Share</SoftButton>
        </div>
      </div>

      {/* year pills */}
      <div style={{ display: 'flex', gap: 6, marginTop: 20, overflowX: 'auto', paddingBottom: 4 }}>
        {YEARS.map((y, i) => (
          <SoftButton key={i} tone={y.y === 2026 ? 'claret' : 'ghost'}>
            <span>{y.y}</span>
            <span style={{ fontSize: 10, fontWeight: 500, opacity: 0.65, marginLeft: 6 }}>{y.c}</span>
          </SoftButton>
        ))}
      </div>

      {/* hero + side events */}
      <div style={{ marginTop: 24, display: 'grid', gridTemplateColumns: '1.7fr 1fr', gap: 14 }}>
        <SoftFabric tone="butter" radius={22} style={{ padding: 8 }} pressable>
          <div style={{ borderRadius: 17, overflow: 'hidden', boxShadow: 'inset 0 0 0 0.5px rgba(40,15,10,0.14)' }}>
            <Photo tone="butter" label="SUNDAY LUNCH · MARGOT'S KITCHEN" ratio={16/9} mode={mode} />
          </div>
          <div style={{ padding: '12px 10px 6px', display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{ flex: 1 }}>
              <div style={{ fontFamily: R_FONT, fontSize: 22, fontWeight: 700, color: INK.ink, lineHeight: 1.1, letterSpacing: -0.3 }}>Sunday lunch</div>
              <div style={{ fontFamily: R_FONT, fontSize: 12, fontWeight: 500, color: INK.soft, marginTop: 3 }}>Apr 12 · Margot's kitchen · 24 photos · 6 people</div>
            </div>
            <div style={{ display: 'flex' }}>
              {MEMBERS.slice(0, 4).map((m, i) => (
                <div key={i} style={{ marginLeft: i === 0 ? 0 : -8 }}>
                  <AvatarR name={m.n} tone={m.tone} size={28} />
                </div>
              ))}
            </div>
          </div>
        </SoftFabric>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {EVENTS.slice(1, 3).map((e, i) => (
            <SoftFabric key={i} tone={e.tone} radius={18} style={{ padding: 6, flex: 1, display: 'flex', gap: 12 }} pressable>
              <div style={{ width: 120, flexShrink: 0, borderRadius: 13, overflow: 'hidden' }}>
                <Photo tone={e.tone} ratio={1} mode={mode} />
              </div>
              <div style={{ padding: '10px 10px 10px 0', display: 'flex', flexDirection: 'column', justifyContent: 'center' }}>
                <div style={{ fontFamily: R_FONT, fontSize: 15, fontWeight: 700, lineHeight: 1.1 }}>{e.t}</div>
                <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 500, opacity: 0.7, marginTop: 5 }}>{e.sub}</div>
                <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 500, opacity: 0.7, marginTop: 2 }}>{e.count} · {e.people} people</div>
              </div>
            </SoftFabric>
          ))}
        </div>
      </div>

      {/* month section */}
      <div style={{ marginTop: 30 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', paddingBottom: 14, borderBottom: '0.5px solid rgba(80,35,20,0.18)' }}>
          <div>
            <div style={{ fontFamily: R_FONT, fontSize: 22, fontWeight: 700, color: INK.ink }}>April</div>
            <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 500, color: INK.soft, marginTop: 3 }}>72 memories across 3 events</div>
          </div>
          <div style={{ fontFamily: R_FONT, fontSize: 12, fontWeight: 500, color: INK.mute }}>Labeled by Clara, Theo, Margot</div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 5, marginTop: 14 }}>
          {['butter','clay','mustard','moss','sky','rose','terra','claret','lavender','pine','sky','butter','mustard','clay'].map((tn, i) => (
            <div key={i} style={{ borderRadius: 9, overflow: 'hidden', boxShadow: 'inset 0 0 0 0.5px rgba(40,15,10,0.12), 0 2px 6px -2px rgba(60,20,10,0.15)' }}>
              <Photo tone={tn} ratio={1} mode={mode} />
            </div>
          ))}
        </div>
      </div>
    </>
  );
}

window.IPadFrame = IPadFrame;
window.IPadApp = IPadApp;
