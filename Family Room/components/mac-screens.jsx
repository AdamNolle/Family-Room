// Mac — three-pane cozy glass with soft fabric right inspector.

function MacFrame({ width = 1300, height = 840, children }) {
  return (
    <div style={{
      width, height, borderRadius: 14, overflow: 'hidden', position: 'relative',
      boxShadow: '0 36px 80px rgba(60,20,10,0.28), 0 0 0 0.5px rgba(0,0,0,0.25)',
    }}>{children}</div>
  );
}

function MacApp({ mode = 'natural', intensity = 1 }) {
  const base = mode === 'daylight' ? '#f5ddb4' : '#fcf2d9';
  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', background: base, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
      <WindowLight mode={mode} intensity={intensity} />

      {/* titlebar */}
      <div style={{ position: 'relative', zIndex: 3, height: 38, display: 'flex', alignItems: 'center', padding: '0 14px', flexShrink: 0 }}>
        <div style={{ display: 'flex', gap: 8 }}>
          {['#ff5f57','#febc2e','#28c840'].map((c, i) => (
            <div key={i} style={{ width: 12, height: 12, borderRadius: 6, background: c, boxShadow: 'inset 0 0 0 0.5px rgba(0,0,0,0.18), inset 0 1px 0 rgba(255,255,255,0.35)' }} />
          ))}
        </div>
        <div style={{ flex: 1, textAlign: 'center', fontFamily: R_FONT, fontSize: 12.5, fontWeight: 600, color: INK.soft }}>
          The Alderhouse · Family Room
        </div>
      </div>

      <div style={{ position: 'relative', zIndex: 2, flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* Sidebar */}
        <div style={{ width: 238, padding: '4px 10px 14px', display: 'flex', flexDirection: 'column', gap: 3, flexShrink: 0, overflow: 'auto' }}>
          <SideGroup title="Library">
            <SideRow icon={Icon.home} label="Everything" n="4,218" />
            <SideRow icon={Icon.calendar} label="On this day" n="18" />
            <SideRow icon={Icon.heartFill} label="Loved" n="312" />
            <SideRow icon={Icon.bookmark} label="Recently added" n="86" />
          </SideGroup>
          <SideGroup title="Organize">
            <SideRow icon={Icon.calendar} label="Years" n="8" />
            <SideRow icon={Icon.people} label="People" n="8" on />
            <SideRow icon={Icon.sparkle} label="Events" n="142" />
            <SideRow icon={Icon.tag} label="Places" n="14" />
            <SideRow icon={Icon.book} label="Albums" n="26" />
          </SideGroup>
          <SideGroup title="People">
            {MEMBERS.slice(0, 5).map((m, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '5px 10px', borderRadius: 9 }}>
                <AvatarR name={m.n} tone={m.tone} size={20} />
                <div style={{ flex: 1, fontFamily: R_FONT, fontSize: 12.5, fontWeight: 500, color: INK.soft }}>{m.n}</div>
                <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 500, color: INK.mute }}>{m.count}</div>
              </div>
            ))}
          </SideGroup>
        </div>

        {/* Center */}
        <div style={{ flex: 1, overflow: 'auto', padding: '4px 12px 14px 4px' }}>
          <SoftFabric tone="paper" radius={20} raised={false} style={{ padding: '24px 26px', minHeight: '100%' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end', marginBottom: 18 }}>
              <div>
                <div style={{ fontFamily: R_FONT, fontSize: 11, fontWeight: 700, letterSpacing: 1.8, color: INK.mute, textTransform: 'uppercase' }}>Organize · People</div>
                <div style={{ fontFamily: R_FONT, fontSize: 34, color: INK.ink, fontWeight: 700, lineHeight: 1, marginTop: 4, letterSpacing: -0.6 }}>Everyone in the room</div>
                <div style={{ fontFamily: R_FONT, fontSize: 12.5, fontWeight: 500, color: INK.soft, marginTop: 6 }}>8 members · 4,218 memories · sorted by contributions</div>
              </div>
              <div style={{ display: 'flex', gap: 7 }}>
                <SoftButton tone="ghost">Grid</SoftButton>
                <SoftButton tone="ghost">A → Z</SoftButton>
              </div>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 12 }}>
              {MEMBERS.map((m, i) => (
                <SoftFabric key={i} tone={m.tone} radius={18} style={{ padding: 6 }} pressable>
                  <div style={{ borderRadius: 13, overflow: 'hidden' }}>
                    <Photo tone={m.tone} label={m.n} ratio={1} mode={mode} />
                  </div>
                  <div style={{ padding: '10px 6px 4px', display: 'flex', alignItems: 'center', gap: 9 }}>
                    <AvatarR name={m.n} tone={m.tone} size={24} />
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontFamily: R_FONT, fontSize: 13.5, fontWeight: 700, lineHeight: 1.1 }}>{m.n}</div>
                      <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 500, opacity: 0.7, marginTop: 2 }}>{m.r} · {m.count}</div>
                    </div>
                  </div>
                </SoftFabric>
              ))}
            </div>

            {/* year bars */}
            <div style={{ marginTop: 30 }}>
              <div style={{ fontFamily: R_FONT, fontSize: 18, fontWeight: 700, color: INK.ink, marginBottom: 14 }}>Memories across the years</div>
              <div style={{ display: 'flex', gap: 8, alignItems: 'flex-end', height: 140 }}>
                {YEARS.map((y, i) => (
                  <div key={i} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
                    <div style={{
                      width: '100%', height: Math.max(22, y.c / 10),
                      background: `linear-gradient(180deg, #e38c6e 0%, #8f3a3a 100%)`,
                      borderRadius: '8px 8px 3px 3px',
                      opacity: y.y === 2026 ? 1 : 0.7,
                      boxShadow: 'inset 0 1.5px 0 rgba(255,230,200,0.45), inset 0 -2px 3px rgba(40,10,5,0.25), 0 3px 8px -2px rgba(120,40,30,0.35)',
                    }} />
                    <div style={{ fontFamily: R_FONT, fontSize: 12, fontWeight: 700, color: y.y === 2026 ? INK.ink : INK.soft }}>{y.y}</div>
                    <div style={{ fontFamily: R_FONT, fontSize: 10, fontWeight: 500, color: INK.mute }}>{y.c}</div>
                  </div>
                ))}
              </div>
            </div>
          </SoftFabric>
        </div>

        {/* Right inspector */}
        <div style={{ width: 310, padding: '4px 10px 14px 0', overflow: 'auto' }}>
          <SoftFabric tone="claret" radius={20} style={{ minHeight: '100%', padding: 20 }}>
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 9 }}>
              <AvatarR name="Iris" tone="clay" size={82} />
              <div style={{ fontFamily: R_FONT, fontSize: 22, fontWeight: 700, color: '#f4dfb2' }}>Iris</div>
              <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 700, color: 'rgba(246,224,200,0.68)', letterSpacing: 1.4, textTransform: 'uppercase' }}>Daughter · since 2020</div>
            </div>

            <div style={{ marginTop: 18, display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 7 }}>
              {[{ l: 'Moments', n: '1,022' }, { l: 'Events', n: '68' }, { l: 'Loved', n: '142' }].map((s, i) => (
                <div key={i} style={{
                  textAlign: 'center', padding: '10px 6px', borderRadius: 12,
                  background: 'rgba(244,223,178,0.15)',
                  boxShadow: 'inset 0 1px 0 rgba(255,240,210,0.22)',
                }}>
                  <div style={{ fontFamily: R_FONT, fontSize: 18, fontWeight: 700, color: '#f4dfb2' }}>{s.n}</div>
                  <div style={{ fontFamily: R_FONT, fontSize: 10, fontWeight: 500, color: 'rgba(246,224,200,0.68)', marginTop: 1 }}>{s.l}</div>
                </div>
              ))}
            </div>

            <div style={{ fontFamily: R_FONT, fontSize: 13, fontWeight: 700, color: '#f4dfb2', marginTop: 22, marginBottom: 8 }}>Recent in Iris's story</div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 5 }}>
              {['clay','rose','mustard','butter'].map((tn, i) => (
                <div key={i} style={{ borderRadius: 9, overflow: 'hidden' }}>
                  <Photo tone={tn} ratio={1} mode={mode} />
                </div>
              ))}
            </div>

            <div style={{ fontFamily: R_FONT, fontSize: 13, fontWeight: 700, color: '#f4dfb2', marginTop: 22, marginBottom: 8 }}>Often with</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
              {MEMBERS.slice(2, 5).map((m, i) => (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '8px 10px', borderRadius: 11, background: 'rgba(244,223,178,0.12)' }}>
                  <AvatarR name={m.n} tone={m.tone} size={26} />
                  <div style={{ flex: 1 }}>
                    <div style={{ fontFamily: R_FONT, fontSize: 12.5, fontWeight: 700, color: '#f4dfb2' }}>{m.n}</div>
                    <div style={{ fontFamily: R_FONT, fontSize: 10.5, fontWeight: 500, color: 'rgba(246,224,200,0.65)' }}>{Math.floor(m.count * 0.4)} shared</div>
                  </div>
                </div>
              ))}
            </div>

            <div style={{ fontFamily: R_FONT, fontSize: 13, fontWeight: 700, color: '#f4dfb2', marginTop: 22, marginBottom: 8 }}>Favorite places</div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
              {['Lake house', 'Back road', "Margot's kitchen", 'Garden'].map((p, i) => (
                <div key={i} style={{
                  padding: '6px 11px', borderRadius: 999,
                  background: 'rgba(244,223,178,0.18)',
                  color: '#f4dfb2',
                  fontFamily: R_FONT, fontSize: 11, fontWeight: 600,
                  boxShadow: 'inset 0 1px 0 rgba(255,240,210,0.2)',
                }}>{p}</div>
              ))}
            </div>
          </SoftFabric>
        </div>
      </div>
    </div>
  );
}

window.MacFrame = MacFrame;
window.MacApp = MacApp;
