// People Identifier — face library, training, merging, suggestions.
// Two-column: left lists "people" (faces clustered), right is review queue.

function PeopleIdentifier() {
  const knownPeople = [
    { ...PEOPLE[0], count: 892,  trained: 'Trained' },
    { ...PEOPLE[1], count: 654,  trained: 'Trained' },
    { ...PEOPLE[2], count: 1204, trained: 'Trained' },
    { ...PEOPLE[3], count: 743,  trained: 'Trained' },
    { ...PEOPLE[4], count: 2108, trained: 'Trained' },
  ];
  const unnamed = [
    { id: 'u1', count: 47, hint: 'Often with Iris' },
    { id: 'u2', count: 23, hint: 'At the lake house' },
    { id: 'u3', count: 12, hint: 'School events' },
  ];

  return (
    <FeatureShell feature="People">
      <SectionHead
        kicker="Who's in your photos"
        title="Faces, sorted with care"
        sub="On-device recognition groups faces into people — you confirm or correct. Nothing leaves your library, and you can turn this off in Settings.">
        <NeuButton icon="merge">Merge people</NeuButton>
        <NeuButton icon="sparkles" primary>Run a fresh scan</NeuButton>
      </SectionHead>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16, minHeight: 0 }}>

        {/* ── Known people ──────────────────────────────── */}
        <section aria-label="Known people" style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
            <Icon name="people" size={16} color="#4a4035"/>
            <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>Named · 5</div>
            <div style={{ flex: 1 }}/>
            <NeuChip active>All</NeuChip>
            <NeuChip>Family</NeuChip>
            <NeuChip>Friends</NeuChip>
          </div>

          <div style={{ flex: 1, padding: 14, borderRadius: 16, background: CREAM,
            boxShadow: NEU_IN_SM, overflow: 'auto' }}>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
              {knownPeople.map(p => (
                <article key={p.id} style={{ padding: 12, borderRadius: 14,
                  background: CREAM, boxShadow: NEU_OUT_SM,
                  display: 'flex', flexDirection: 'column', gap: 10 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <Avatar p={p} size={36}/>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>{p.full}</div>
                      <div style={{ fontSize: 11, color: '#6e6358' }}>{p.count.toLocaleString()} photos</div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 4,
                      padding: '3px 8px', borderRadius: 999,
                      background: CREAM, boxShadow: NEU_IN_SM,
                      fontSize: 10, fontWeight: 600, color: '#4a7a4a' }}>
                      <Icon name="check" size={10} color="#4a7a4a" strokeWidth={2.4}/>
                      {p.trained}
                    </div>
                  </div>
                  {/* Sample faces */}
                  <div style={{ display: 'flex', gap: 4 }}>
                    {['indoor','garden','lake','sunset','cake'].map((s,i) => (
                      <div key={i} style={{ flex: 1, position: 'relative' }}>
                        <Photo scene={s} h={42} round={6}/>
                      </div>
                    ))}
                  </div>
                </article>
              ))}

              {/* Add person tile */}
              <button style={{ padding: 12, borderRadius: 14, border: 'none',
                background: CREAM, boxShadow: NEU_IN_SM, cursor: 'pointer',
                display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
                gap: 8, minHeight: 110 }}>
                <div style={{ width: 36, height: 36, borderRadius: 18, background: CREAM,
                  boxShadow: NEU_OUT_SM, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <Icon name="plus" size={18} color="#b86348"/>
                </div>
                <div style={{ fontSize: 12, fontWeight: 600, color: '#b86348' }}>Add a person</div>
                <div style={{ fontSize: 10.5, color: '#6e6358' }}>pick a face to start training</div>
              </button>
            </div>
          </div>
        </section>

        {/* ── Review queue ──────────────────────────────── */}
        <section aria-label="Review queue" style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
            <Icon name="tag" size={16} color="#4a4035"/>
            <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>Needs your review · 3</div>
          </div>

          <div style={{ flex: 1, padding: 14, borderRadius: 16, background: CREAM,
            boxShadow: NEU_IN_SM, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 14 }}>

            {/* Featured suggestion */}
            <div style={{ padding: 14, borderRadius: 14, background: CREAM, boxShadow: NEU_OUT_SM }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
                <Icon name="sparkles" size={14} color="#b86348"/>
                <div style={{ fontSize: 11.5, fontWeight: 600, color: '#b86348',
                  textTransform: 'uppercase', letterSpacing: 0.8 }}>Suggestion · 47 photos</div>
              </div>
              <div style={{ display: 'flex', gap: 10, alignItems: 'center', marginBottom: 12 }}>
                <Photo scene="garden" w={80} h={80} round={12}/>
                <div>
                  <div style={{ fontFamily: DISPLAY, fontSize: 18, fontWeight: 500, color: '#2a221a',
                    letterSpacing: -0.3 }}>Looks like one person.</div>
                  <div style={{ fontSize: 12.5, color: '#4a4035', marginTop: 4, lineHeight: 1.45 }}>
                    Often with Iris. Probably someone from her preschool. Want to give them a name?
                  </div>
                </div>
              </div>
              <div style={{ display: 'flex', gap: 4, marginBottom: 12 }}>
                {['garden','indoor','bike','stage','snow','fall'].map((s,i) => (
                  <Photo key={i} scene={s} w={42} h={42} round={6}/>
                ))}
                <div style={{ width: 42, height: 42, borderRadius: 6, background: CREAM,
                  boxShadow: NEU_IN_SM, display: 'flex', alignItems: 'center', justifyContent: 'center',
                  fontSize: 11, fontWeight: 600, color: '#6e6358' }}>+41</div>
              </div>
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                <input placeholder="Name them…" style={{ flex: 1, boxSizing: 'border-box',
                  padding: '9px 12px', borderRadius: 10, border: 'none',
                  background: CREAM, boxShadow: NEU_IN_SM, fontSize: 13, fontFamily: SANS,
                  color: '#2a221a', outline: 'none' }}/>
                <NeuButton size="sm">Skip</NeuButton>
                <NeuButton size="sm" primary icon="check">Save</NeuButton>
              </div>
            </div>

            {/* Smaller suggestions */}
            {unnamed.slice(1).map(u => (
              <div key={u.id} style={{ padding: 12, borderRadius: 14, background: CREAM, boxShadow: NEU_OUT_SM,
                display: 'flex', alignItems: 'center', gap: 12 }}>
                <Photo scene={u.id === 'u2' ? 'lake' : 'studio'} w={56} h={56} round={10}/>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>{u.count} photos</div>
                  <div style={{ fontSize: 11.5, color: '#6e6358', marginTop: 2 }}>{u.hint}</div>
                </div>
                <button aria-label="Merge into existing person" style={{ width: 32, height: 32,
                  borderRadius: 10, border: 'none', background: CREAM, boxShadow: NEU_OUT_SM,
                  cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <Icon name="merge" size={16} color="#4a4035"/>
                </button>
                <NeuButton size="sm" icon="tag">Name</NeuButton>
              </div>
            ))}

            {/* "Is this Iris?" merge prompt */}
            <div style={{ padding: 12, borderRadius: 14, background: CREAM, boxShadow: NEU_OUT_SM }}>
              <div style={{ fontSize: 11.5, fontWeight: 600, color: '#6e6358',
                textTransform: 'uppercase', letterSpacing: 0.8, marginBottom: 10 }}>Possible match</div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <Photo scene="stage" w={48} h={48} round={10}/>
                <div style={{ display: 'flex', alignItems: 'center', gap: 4, color: '#6e6358', fontSize: 18 }}>≈</div>
                <Avatar p={PEOPLE[4]} size={44}/>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: '#2a221a' }}>Is this <strong>Iris</strong>?</div>
                  <div style={{ fontSize: 11, color: '#6e6358', marginTop: 2 }}>From the school recital, 12 photos</div>
                </div>
                <NeuButton size="sm">No</NeuButton>
                <NeuButton size="sm" primary icon="check">Yes, merge</NeuButton>
              </div>
            </div>
          </div>
        </section>
      </div>
    </FeatureShell>
  );
}

window.PeopleIdentifier = PeopleIdentifier;
