// Family manager — multiple families, friend groups, roles, invites.

function FamilyManager() {
  const families = [
    { id: 'alder', name: 'The Alderhouse', sub: 'Margot, Henry & co', role: 'Admin', count: 5,
      tone: '#c97a5d', accent: '#f0bfa8', active: true,
      members: PEOPLE,
    },
    { id: 'mor', name: 'The Morimotos', sub: "Cora's in-laws", role: 'Member', count: 4,
      tone: '#7a6a8a', accent: '#c4b8d8',
      members: [
        { id: 'K', full: 'Kenji', tone: '#7a6a8a', accent: '#c4b8d8' },
        { id: 'S', full: 'Saori', tone: '#5a8a7a', accent: '#a8d4c4' },
        { id: 'R', full: 'Ren',   tone: '#8a6a4a', accent: '#d8c0a0' },
        { id: 'Y', full: 'Yui',   tone: '#a85878', accent: '#e8b4c8' },
      ],
    },
    { id: 'col', name: 'Colton cousins', sub: 'Iris\'s extended family', role: 'Member', count: 12,
      tone: '#5a8a6a', accent: '#a8d4b4',
      members: [
        { id: 'B1', full: 'Beatrix', tone: '#5a8a6a', accent: '#a8d4b4' },
        { id: 'P1', full: 'Pete',    tone: '#8a7a4a', accent: '#d4c898' },
        { id: 'L1', full: 'Liv',     tone: '#a86a8a', accent: '#e0b4d0' },
        { id: 'F1', full: 'Felix',   tone: '#5a7a8a', accent: '#a4c4d4' },
      ],
    },
  ];
  const friendGroups = [
    { id: 'la', name: 'Sunday brunch crew', size: 6, last: 'last shared 2 weeks ago',
      tones: ['#c97a5d','#7a8a4a','#4a7a82','#a85878'] },
    { id: 'b',  name: 'Beach club',          size: 9, last: 'shared today',
      tones: ['#4a7a82','#d49060','#7a6a8a','#5a8a6a','#a85878'] },
    { id: 'cw', name: 'College room 4B',     size: 5, last: 'shared 3 days ago',
      tones: ['#7a6a4a','#a85878','#5a7a8a','#8a4a3a'] },
  ];

  return (
    <FeatureShell feature="Settings">
      <SectionHead
        kicker="Family manager"
        title="Your families and friends"
        sub="A person can belong to more than one family — Cora is in The Alderhouse and married into the Morimotos. Friend groups are smaller circles you share specific moments with.">
        <NeuButton icon="invite">Invitations · 2</NeuButton>
        <NeuButton icon="plus" primary>Start a family</NeuButton>
      </SectionHead>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1fr 380px', gap: 18, minHeight: 0 }}>

        {/* ── Families ─────────────────────────────────── */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 14, minHeight: 0, overflow: 'auto', paddingRight: 4 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Icon name="family" size={16} color="#4a4035"/>
            <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>Families · {families.length}</div>
          </div>

          {families.map(f => (
            <article key={f.id}
              aria-label={f.name}
              style={{ padding: 18, borderRadius: 18, background: CREAM,
              boxShadow: NEU_OUT_SM, position: 'relative' }}>
              {f.active && (
                <div style={{ position: 'absolute', top: 14, right: 14,
                  padding: '3px 10px', borderRadius: 999,
                  background: CREAM, boxShadow: NEU_IN_SM,
                  fontSize: 10, fontWeight: 600, color: '#4a7a4a',
                  textTransform: 'uppercase', letterSpacing: 0.8,
                  display: 'flex', alignItems: 'center', gap: 4 }}>
                  <div style={{ width: 6, height: 6, borderRadius: 3, background: '#4a7a4a',
                    boxShadow: '0 0 4px rgba(74,122,74,0.6)' }}/>
                  Active
                </div>
              )}

              <div style={{ display: 'flex', alignItems: 'flex-start', gap: 14 }}>
                {/* Crest */}
                <div style={{ width: 56, height: 56, borderRadius: 14,
                  background: `linear-gradient(135deg, ${f.accent}, ${f.tone})`,
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  fontFamily: DISPLAY, fontSize: 22, fontWeight: 600, color: '#fff',
                  boxShadow: '0 2px 6px rgba(40,30,20,0.18)' }}>
                  {f.name.replace(/^The /, '')[0]}
                </div>

                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontFamily: DISPLAY, fontSize: 22, fontWeight: 500, color: '#2a221a',
                    letterSpacing: -0.4 }}>{f.name}</div>
                  <div style={{ fontSize: 12.5, color: '#6e6358', marginTop: 2 }}>
                    {f.sub} · {f.count} {f.count === 1 ? 'member' : 'members'}
                  </div>

                  {/* Members row */}
                  <div style={{ display: 'flex', alignItems: 'center', marginTop: 12, gap: 4 }}>
                    {f.members.slice(0,5).map((p,i) => (
                      <div key={p.id} style={{ marginLeft: i === 0 ? 0 : -8 }}>
                        <Avatar p={p} size={30} ring/>
                      </div>
                    ))}
                    {f.members.length > 5 && (
                      <div style={{ marginLeft: -8, width: 30, height: 30, borderRadius: 15,
                        background: CREAM, boxShadow: NEU_IN_SM,
                        display: 'flex', alignItems: 'center', justifyContent: 'center',
                        fontSize: 11, fontWeight: 600, color: '#6e6358',
                        border: '2px solid ' + CREAM }}>
                        +{f.members.length - 5}
                      </div>
                    )}
                    <div style={{ flex: 1 }}/>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 4,
                      padding: '4px 10px', borderRadius: 999,
                      background: CREAM, boxShadow: NEU_IN_SM,
                      fontSize: 11, fontWeight: 600, color: '#2a221a' }}>
                      <Icon name={f.role === 'Admin' ? 'role-admin' : 'people'} size={12} color={f.role === 'Admin' ? '#b86348' : '#4a4035'}/>
                      {f.role}
                    </div>
                  </div>

                  {/* Action row */}
                  <div style={{ display: 'flex', gap: 8, marginTop: 14, flexWrap: 'wrap' }}>
                    <NeuButton size="sm" icon="invite">Invite</NeuButton>
                    <NeuButton size="sm" icon="people">Members</NeuButton>
                    <NeuButton size="sm" icon="cog">Settings</NeuButton>
                    {!f.active && <NeuButton size="sm" icon="dot" primary>Switch to</NeuButton>}
                    <div style={{ flex: 1 }}/>
                    {f.role !== 'Admin' && (
                      <button aria-label={`Leave ${f.name}`} style={{ padding: '7px 12px', borderRadius: 10,
                        border: 'none', background: 'transparent',
                        fontSize: 11.5, color: '#8a7e70', cursor: 'pointer', fontFamily: SANS }}>
                        Leave family
                      </button>
                    )}
                  </div>
                </div>
              </div>
            </article>
          ))}

          {/* Pending invitations */}
          <div style={{ marginTop: 4, padding: 14, borderRadius: 16, background: CREAM,
            boxShadow: NEU_IN_SM }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
              <Icon name="invite" size={14} color="#b86348"/>
              <div style={{ fontSize: 11.5, fontWeight: 600, color: '#b86348',
                textTransform: 'uppercase', letterSpacing: 0.8 }}>Pending · 2</div>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {[
                { from: 'Aunt Beatrix', what: 'invited you to The Coltons' },
                { from: 'Daniela',      what: 'wants to share an album with you' },
              ].map((i,n) => (
                <div key={n} style={{ display: 'flex', alignItems: 'center', gap: 10,
                  padding: 10, borderRadius: 12, background: CREAM, boxShadow: NEU_OUT_SM }}>
                  <div style={{ width: 32, height: 32, borderRadius: 16,
                    background: 'linear-gradient(135deg, #d4c298, #7a6a4a)',
                    color: '#fff', fontWeight: 600, display: 'flex',
                    alignItems: 'center', justifyContent: 'center', fontSize: 13 }}>
                    {i.from[0]}
                  </div>
                  <div style={{ flex: 1, fontSize: 12.5, color: '#2a221a' }}>
                    <strong>{i.from}</strong> {i.what}
                  </div>
                  <NeuButton size="sm">Decline</NeuButton>
                  <NeuButton size="sm" primary icon="check">Accept</NeuButton>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* ── Friend groups ────────────────────────────── */}
        <aside aria-label="Friend groups" style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
            <Icon name="friends" size={16} color="#4a4035"/>
            <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>Friend groups</div>
            <div style={{ flex: 1 }}/>
            <button aria-label="New group" style={{ width: 28, height: 28, borderRadius: 8,
              border: 'none', background: CREAM, boxShadow: NEU_OUT_SM, cursor: 'pointer',
              display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <Icon name="plus" size={14} color="#b86348"/>
            </button>
          </div>

          <div style={{ flex: 1, padding: 14, borderRadius: 16, background: CREAM,
            boxShadow: NEU_IN_SM, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
            {friendGroups.map(g => (
              <article key={g.id} style={{ padding: 14, borderRadius: 14,
                background: CREAM, boxShadow: NEU_OUT_SM }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginBottom: 8 }}>
                  {g.tones.slice(0,4).map((t,i) => (
                    <div key={i} style={{ marginLeft: i === 0 ? 0 : -8,
                      width: 26, height: 26, borderRadius: 13,
                      background: `linear-gradient(135deg, ${t}cc, ${t})`,
                      border: `2px solid ${CREAM}` }}/>
                  ))}
                  {g.size > 4 && (
                    <div style={{ marginLeft: -8, width: 26, height: 26, borderRadius: 13,
                      background: CREAM, boxShadow: NEU_IN_SM,
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      fontSize: 10, fontWeight: 600, color: '#6e6358',
                      border: `2px solid ${CREAM}` }}>+{g.size - 4}</div>
                  )}
                </div>
                <div style={{ fontSize: 13.5, fontWeight: 600, color: '#2a221a' }}>{g.name}</div>
                <div style={{ fontSize: 11, color: '#6e6358', marginTop: 2 }}>
                  {g.size} people · {g.last}
                </div>
                <div style={{ display: 'flex', gap: 6, marginTop: 10 }}>
                  <NeuButton size="sm" icon="album">Share album</NeuButton>
                  <NeuButton size="sm" icon="more"></NeuButton>
                </div>
              </article>
            ))}

            <div style={{ marginTop: 4, padding: 14, borderRadius: 14,
              background: CREAM, boxShadow: NEU_IN_SM, fontSize: 12, color: '#4a4035', lineHeight: 1.5 }}>
              <div style={{ fontSize: 11.5, fontWeight: 600, color: '#6e6358',
                textTransform: 'uppercase', letterSpacing: 0.8, marginBottom: 6 }}>Tip</div>
              Friend groups are quieter than families — no auto-reels, no notifications by default. Use them for the people you want to share certain moments with, not your whole life.
            </div>
          </div>
        </aside>
      </div>
    </FeatureShell>
  );
}

window.FamilyManager = FamilyManager;
