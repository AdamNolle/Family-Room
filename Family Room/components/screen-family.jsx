// Family & Memory screens

// ═══════════════════════════════════════════════════════════════
// SCREEN 5 — FAMILY / MEMBERS
// ═══════════════════════════════════════════════════════════════
function FamilyScreen({ woodTone = 'walnut', warmth = 'afternoon', intensity = 1 }) {
  return (
    <ScreenShell warmth={warmth} intensity={intensity} bg="#f5e7ce">
      <HeaderPad>
        <div style={{ padding: '0 20px' }}>
          <div style={{ fontFamily: 'system-ui', fontSize: 13, color: '#8a6540', letterSpacing: 1.5, textTransform: 'uppercase' }}>
            The Alderhouse
          </div>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 40, color: '#2a1a10', lineHeight: 1, marginTop: 6, fontWeight: 400 }}>
            Family
          </div>
          <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#7a5a3a', marginTop: 6 }}>
            Eight members · since 2019
          </div>
        </div>

        {/* wooden frame with grouped portraits */}
        <div style={{ padding: '22px 14px 0' }}>
          <WoodSurface tone={woodTone} radius={24} style={{ padding: 16, boxShadow: '0 16px 40px -10px rgba(60,30,10,0.5)' }}>
            <LinenSurface tone="cream" radius={14} style={{ padding: 14 }}>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
                {MEMBERS.slice(0,4).map((m, i) => (
                  <MemberCard key={i} member={m} />
                ))}
              </div>
            </LinenSurface>
          </WoodSurface>
        </div>

        {/* pending invites */}
        <div style={{ padding: '24px 22px 0' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
            <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 20, color: '#2a1a10' }}>Everyone else</div>
            <div style={{ fontFamily: 'system-ui', fontSize: 13, color: '#9a7040' }}>+ Invite</div>
          </div>
        </div>

        <div style={{ padding: '14px 22px 0', display: 'flex', flexDirection: 'column', gap: 10 }}>
          {MEMBERS.slice(4).map((m, i) => <MemberRow key={i} m={m} />)}

          {/* invite pending */}
          <div style={{
            display: 'flex', alignItems: 'center', gap: 12, padding: '10px 12px',
            borderRadius: 16, background: 'rgba(255,248,234,0.5)',
            border: '1px dashed rgba(140,90,50,0.3)',
          }}>
            <div style={{
              width: 42, height: 42, borderRadius: 21, border: '1.5px dashed rgba(140,90,50,0.45)',
              display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#8a6540',
              fontFamily: '"Fraunces", Georgia, serif', fontSize: 18,
            }}>?</div>
            <div style={{ flex: 1 }}>
              <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#5a2a0a', fontWeight: 500 }}>
                Uncle Sam
              </div>
              <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#8a6540' }}>
                Invite sent 2 days ago
              </div>
            </div>
            <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#9a7040' }}>Resend</div>
          </div>
        </div>

        {/* room settings */}
        <div style={{ padding: '30px 22px 120px' }}>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 20, color: '#2a1a10', marginBottom: 12 }}>
            Room settings
          </div>
          <LinenSurface tone="oat" radius={20}>
            <SettingRow label="Who can invite" value="Parents only" />
            <SettingRow label="Who can delete" value="Only the poster" />
            <SettingRow label="Shared with" value="This device + iCloud" last />
          </LinenSurface>
        </div>
      </HeaderPad>
    </ScreenShell>
  );
}

function MemberCard({ member }) {
  return (
    <div style={{
      background: '#fff8ea', borderRadius: 14, padding: 14,
      boxShadow: '0 2px 6px rgba(80,40,10,0.08), 0 0 0 0.5px rgba(140,90,50,0.1)',
      display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8,
    }}>
      <Avatar name={member.n} hue={member.hue} size={56} />
      <div style={{ textAlign: 'center' }}>
        <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 17, color: '#2a1a10' }}>{member.n}</div>
        <div style={{ fontFamily: 'system-ui', fontSize: 11, color: '#8a6540', marginTop: 2, letterSpacing: 0.5 }}>{member.r.toUpperCase()}</div>
      </div>
    </div>
  );
}

function MemberRow({ m }) {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 12, padding: '8px 12px',
      borderRadius: 16, background: 'rgba(255,248,234,0.7)',
      boxShadow: '0 0 0 0.5px rgba(140,90,50,0.08)',
    }}>
      <Avatar name={m.n} hue={m.hue} size={42} />
      <div style={{ flex: 1 }}>
        <div style={{ fontFamily: 'system-ui', fontSize: 15, color: '#2a1a10', fontWeight: 500 }}>{m.n}</div>
        <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#8a6540' }}>{m.r} · contributed 142 moments</div>
      </div>
      {Icon.chevronR(16, '#b08860')}
    </div>
  );
}

function SettingRow({ label, value, last }) {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', justifyContent: 'space-between',
      padding: '14px 16px',
      borderBottom: last ? 'none' : '0.5px solid rgba(140,90,50,0.15)',
    }}>
      <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#2a1a10' }}>{label}</div>
      <div style={{ fontFamily: 'system-ui', fontSize: 13, color: '#8a6540', display: 'flex', gap: 6, alignItems: 'center' }}>
        {value} {Icon.chevronR(14, '#b08860')}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════
// SCREEN 6 — MEMORY / ON THIS DAY
// ═══════════════════════════════════════════════════════════════
function MemoryScreen({ warmth = 'golden', intensity = 1 }) {
  return (
    <ScreenShell warmth={warmth} intensity={intensity} bg="#f2ddb8">
      <HeaderPad>
        <div style={{ padding: '0 20px' }}>
          <div style={{ fontFamily: 'system-ui', fontSize: 13, color: '#8a5020', letterSpacing: 1.5, textTransform: 'uppercase' }}>
            On this day
          </div>
          <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 38, color: '#2a1a10', lineHeight: 1.05, marginTop: 4, fontStyle: 'italic', fontWeight: 400 }}>
            Three Aprils ago
          </div>
          <div style={{ fontFamily: 'system-ui', fontSize: 14, color: '#7a4818', marginTop: 8, lineHeight: 1.5 }}>
            The afternoon Henry first came home from the hospital. 18 photos and 2 videos from Margot and Clara.
          </div>
        </div>

        {/* large hero photo in frame */}
        <div style={{ padding: '24px 20px 0' }}>
          <WoodSurface tone="walnut" radius={14} style={{ padding: 10, boxShadow: '0 24px 50px -15px rgba(60,30,10,0.5)' }}>
            <LinenSurface tone="cream" radius={4} style={{ padding: 8 }}>
              <div style={{ borderRadius: 2, overflow: 'hidden' }}>
                <PhotoPlaceholder label="APRIL 20 · 2023 · HOME" hue={35} ratio={4/5} />
              </div>
              <div style={{
                fontFamily: '"Fraunces", Georgia, serif', fontStyle: 'italic', fontSize: 13,
                color: '#6a4a28', textAlign: 'center', padding: '10px 0 2px',
              }}>
                "Welcome home, Dad."
              </div>
            </LinenSurface>
          </WoodSurface>
        </div>

        {/* filmstrip */}
        <div style={{ padding: '22px 0 0' }}>
          <div style={{ padding: '0 22px', fontFamily: '"Fraunces", Georgia, serif', fontSize: 18, color: '#2a1a10', marginBottom: 12 }}>
            The rest of that day
          </div>
          <div style={{
            display: 'flex', gap: 10, overflowX: 'auto', padding: '0 20px 8px',
          }}>
            {[40, 28, 60, 180, 12, 100, 200].map((h, i) => (
              <div key={i} style={{
                width: 110, flexShrink: 0, borderRadius: 12, overflow: 'hidden',
                boxShadow: '0 6px 16px rgba(80,40,10,0.18), 0 0 0 0.5px rgba(140,90,50,0.12)',
              }}>
                <PhotoPlaceholder hue={h} ratio={3/4} />
              </div>
            ))}
          </div>
        </div>

        {/* music / ambient note */}
        <div style={{ padding: '24px 20px 0' }}>
          <LinenSurface tone="oat" radius={20} style={{ padding: '14px 16px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <div style={{
                width: 44, height: 44, borderRadius: 22,
                background: 'linear-gradient(160deg, #e8a95a, #b36a2a)',
                display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#fff',
              }}>{Icon.play(20, '#fff')}</div>
              <div style={{ flex: 1 }}>
                <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#8a5020', letterSpacing: 1, textTransform: 'uppercase' }}>
                  Memory reel
                </div>
                <div style={{ fontFamily: '"Fraunces", Georgia, serif', fontSize: 17, color: '#2a1a10', marginTop: 2 }}>
                  A quiet spring afternoon
                </div>
                <div style={{ fontFamily: 'system-ui', fontSize: 12, color: '#7a4818', marginTop: 2 }}>
                  2 min · curated for Clara
                </div>
              </div>
            </div>
          </LinenSurface>
        </div>

        {/* share with family button */}
        <div style={{ padding: '22px 20px 120px' }}>
          <div style={{
            padding: '16px', borderRadius: 22,
            background: 'rgba(255,248,234,0.6)', backdropFilter: 'blur(10px)',
            boxShadow: '0 0 0 0.5px rgba(140,90,50,0.15)',
            display: 'flex', alignItems: 'center', gap: 12,
          }}>
            <div style={{ color: '#8a5020' }}>{Icon.share(22, '#8a5020')}</div>
            <div style={{ flex: 1, fontFamily: 'system-ui', fontSize: 14, color: '#2a1a10' }}>
              Send this memory to the Family Room
            </div>
            {Icon.chevronR(16, '#b08860')}
          </div>
        </div>
      </HeaderPad>
    </ScreenShell>
  );
}

window.FamilyScreen = FamilyScreen;
window.MemoryScreen = MemoryScreen;
