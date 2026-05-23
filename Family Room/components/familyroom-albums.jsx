// Album creator — drag photos in, set cover, invite collaborators.
// Three panes: source library (left), album canvas (center), inspector (right).

function AlbumCreator() {
  const allPhotos = [
    { id: 1,  s: 'lake',   d: 'Jun 2', who: ['M','D','C','T','I'], picked: true },
    { id: 2,  s: 'sunset', d: 'Jun 2', who: ['M','D','I'], picked: true },
    { id: 3,  s: 'garden', d: 'Jun 3', who: ['I','C'], picked: false },
    { id: 4,  s: 'cake',   d: 'Aug 9', who: ['M','D','C','T','I'], picked: true, cover: true },
    { id: 5,  s: 'indoor', d: 'Aug 9', who: ['M','D','C'], picked: true },
    { id: 6,  s: 'fall',   d: 'Oct 11', who: ['M','C'], picked: false },
    { id: 7,  s: 'wolf',   d: 'Oct 31', who: ['I','T'], picked: false },
    { id: 8,  s: 'snow',   d: 'Dec 8', who: ['T','I'], picked: false },
    { id: 9,  s: 'bike',   d: 'Mar 14', who: ['I','D'], picked: true },
    { id: 10, s: 'studio', d: 'May 1', who: ['T'], picked: false },
    { id: 11, s: 'stage',  d: 'Sep 22', who: ['I'], picked: true },
    { id: 12, s: 'table',  d: 'Nov 28', who: ['M','D','C','T','I'], picked: true },
  ];
  const picked = allPhotos.filter(p => p.picked);
  const cover = allPhotos.find(p => p.cover);

  return (
    <FeatureShell feature="Albums">
      <SectionHead
        kicker="New album"
        title="A year together — 2024"
        sub="Drag photos from your library on the left to build the album. The first picture becomes the cover; everyone you tag here can add their own.">
        <NeuButton size="md" icon="share">Share draft</NeuButton>
        <NeuButton size="md" icon="check" primary>Publish · 7 photos</NeuButton>
      </SectionHead>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '300px 1fr 280px',
        gap: 16, minHeight: 0 }}>

        {/* ── Library (source) ──────────────────────────── */}
        <section aria-label="Photo library" style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
            <Icon name="photo" size={16} color="#4a4035"/>
            <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>Library</div>
            <div style={{ fontSize: 11, color: '#6e6358' }}>1,204 photos</div>
          </div>
          <div style={{ display: 'flex', gap: 6, marginBottom: 10, flexWrap: 'wrap' }}>
            <NeuChip active>2024</NeuChip>
            <NeuChip>By Cora</NeuChip>
            <NeuChip>Lake house</NeuChip>
          </div>
          <div style={{ flex: 1, overflow: 'auto', borderRadius: 16,
            background: CREAM, boxShadow: NEU_IN_SM, padding: 10 }}>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 6 }}>
              {allPhotos.map(p => (
                <div key={p.id} style={{ position: 'relative', cursor: 'grab' }}>
                  <Photo scene={p.s} h={64} round={8}/>
                  {p.picked && (
                    <div style={{ position: 'absolute', top: 4, right: 4, width: 16, height: 16,
                      borderRadius: 8, background: '#b86348',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      boxShadow: '0 1px 2px rgba(0,0,0,0.25)' }}>
                      <Icon name="check" size={10} color="#fff" strokeWidth={2.5}/>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* ── Album canvas ──────────────────────────────── */}
        <section aria-label="Album canvas" style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <Icon name="album" size={16} color="#4a4035"/>
              <div style={{ fontSize: 13, fontWeight: 600, color: '#2a221a' }}>Album · 7 photos</div>
            </div>
            <div style={{ display: 'flex', gap: 6 }}>
              <NeuChip icon="sparkles">AI sequence</NeuChip>
              <NeuChip>By date</NeuChip>
            </div>
          </div>

          <div style={{ flex: 1, padding: 16, borderRadius: 16, background: CREAM,
            boxShadow: NEU_IN_SM, overflow: 'auto' }}>
            {/* Cover */}
            <div style={{ position: 'relative', marginBottom: 14 }}>
              <Photo scene={cover.s} h={180} round={14}/>
              <div style={{ position: 'absolute', inset: 0, borderRadius: 14,
                background: 'linear-gradient(180deg, transparent 50%, rgba(20,12,6,0.7) 100%)' }}/>
              <div style={{ position: 'absolute', left: 16, bottom: 14, color: '#fff' }}>
                <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: 1.2,
                  textTransform: 'uppercase', opacity: 0.9 }}>Cover · double-click to change</div>
                <div style={{ fontFamily: DISPLAY, fontSize: 24, fontWeight: 500, marginTop: 2,
                  letterSpacing: -0.4 }}>A year together — 2024</div>
              </div>
            </div>

            {/* Grid of selected */}
            <div style={{ display: 'grid', gridTemplateColumns: '1.6fr 1fr 1fr', gap: 8 }}>
              <div style={{ gridRow: 'span 2' }}><Photo scene="lake" h={188} round={10}/></div>
              <Photo scene="sunset" h={90} round={10}/>
              <Photo scene="indoor" h={90} round={10}/>
              <Photo scene="bike" h={90} round={10}/>
              <Photo scene="stage" h={90} round={10}/>
            </div>

            {/* Drop zone */}
            <div style={{ marginTop: 12, height: 80, borderRadius: 14,
              background: CREAM, boxShadow: NEU_IN_SM,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              gap: 10, color: '#6e6358', fontSize: 13 }}>
              <Icon name="plus" size={18} color="#6e6358"/>
              Drop more photos to add
            </div>
          </div>
        </section>

        {/* ── Inspector ─────────────────────────────────── */}
        <aside aria-label="Album details" style={{ display: 'flex', flexDirection: 'column', gap: 12, minHeight: 0, overflow: 'auto' }}>
          <NeuCard padding={14}>
            <label style={{ fontSize: 11, fontWeight: 600, color: '#6e6358', textTransform: 'uppercase', letterSpacing: 0.6 }}>Title</label>
            <input defaultValue="A year together — 2024"
              style={{ width: '100%', boxSizing: 'border-box', marginTop: 6,
                padding: '9px 12px', borderRadius: 10, border: 'none',
                background: CREAM, boxShadow: NEU_IN_SM, fontFamily: SANS, fontSize: 13,
                color: '#2a221a', outline: 'none' }}/>
            <label style={{ fontSize: 11, fontWeight: 600, color: '#6e6358', textTransform: 'uppercase', letterSpacing: 0.6, display: 'block', marginTop: 12 }}>Caption</label>
            <textarea defaultValue="The big and the small — bike rides, a stage, the lake, Henry's 72nd."
              style={{ width: '100%', boxSizing: 'border-box', marginTop: 6, height: 64,
                padding: '9px 12px', borderRadius: 10, border: 'none', resize: 'none',
                background: CREAM, boxShadow: NEU_IN_SM, fontFamily: SANS, fontSize: 12.5,
                color: '#2a221a', outline: 'none', lineHeight: 1.4 }}/>
          </NeuCard>

          <NeuCard padding={14}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
              <div style={{ fontSize: 12.5, fontWeight: 600, color: '#2a221a' }}>Collaborators</div>
              <button aria-label="Add collaborator" style={{ width: 26, height: 26, border: 'none',
                borderRadius: 8, background: CREAM, boxShadow: NEU_OUT_SM, cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <Icon name="plus" size={14} color="#4a4035"/>
              </button>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {PEOPLE.slice(0,4).map((p,i) => (
                <div key={p.id} style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <Avatar p={p} size={26}/>
                  <div style={{ flex: 1, fontSize: 12.5, color: '#2a221a' }}>{p.full}</div>
                  <div style={{ fontSize: 10.5, fontWeight: 600, color: i === 0 ? '#b86348' : '#6e6358',
                    textTransform: 'uppercase', letterSpacing: 0.6 }}>{i === 0 ? 'Owner' : 'Editor'}</div>
                </div>
              ))}
            </div>
          </NeuCard>

          <NeuCard padding={14}>
            <div style={{ fontSize: 12.5, fontWeight: 600, color: '#2a221a', marginBottom: 8 }}>Visibility</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {[
                { k: 'family',  l: 'The Alderhouse', s: 'Everyone in this family', on: true },
                { k: 'friends', l: 'Plus close friends', s: '8 people from groups' },
                { k: 'private', l: 'Only collaborators', s: '4 people' },
              ].map(o => (
                <label key={o.k} style={{ display: 'flex', alignItems: 'flex-start', gap: 10,
                  padding: 8, borderRadius: 10,
                  boxShadow: o.on ? NEU_IN_SM : 'none', cursor: 'pointer' }}>
                  <div style={{ width: 14, height: 14, borderRadius: 7, marginTop: 2,
                    background: CREAM, boxShadow: o.on ? NEU_IN_SM : NEU_OUT_SM,
                    display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                    {o.on && <div style={{ width: 6, height: 6, borderRadius: 3, background: '#b86348' }}/>}
                  </div>
                  <div>
                    <div style={{ fontSize: 12, fontWeight: 600, color: '#2a221a' }}>{o.l}</div>
                    <div style={{ fontSize: 11, color: '#6e6358', marginTop: 2 }}>{o.s}</div>
                  </div>
                </label>
              ))}
            </div>
          </NeuCard>
        </aside>
      </div>
    </FeatureShell>
  );
}

window.AlbumCreator = AlbumCreator;
