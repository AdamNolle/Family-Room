// DVD nostalgia screen — early-2000s home-video DVD menu vibe.
// Bevels, chrome, looping background scene, scene selection grid, "Play All".
// Wraps the menu in an old TV (faux 4:3 with letterbox + scanlines + corner overlay).

function DvdMenu() {
  // Nine "scenes" from the home video, grouped into chapter cells.
  const chapters = [
    { n: 1, scene: 'lake',   t: 'Pulling into the drive',   tc: '00:00' },
    { n: 2, scene: 'cake',   t: 'Candles & cake',           tc: '04:12' },
    { n: 3, scene: 'garden', t: 'The roses, mid-July',      tc: '09:34' },
    { n: 4, scene: 'sunset', t: 'Boat at dusk',             tc: '15:08', selected: true },
    { n: 5, scene: 'indoor', t: 'The toast',                tc: '21:22' },
    { n: 6, scene: 'bike',   t: "Iris's first bike ride",   tc: '27:46' },
    { n: 7, scene: 'stage',  t: 'School play',              tc: '34:01' },
    { n: 8, scene: 'fall',   t: 'Apple picking',            tc: '41:18' },
    { n: 9, scene: 'studio', t: "Theo's studio",            tc: '47:55' },
  ];

  // Chrome bevel — early-2000s glossy button
  const chrome = (selected = false) => ({
    background: selected
      ? 'linear-gradient(180deg, #ffd87a 0%, #d99838 45%, #a86a1a 55%, #d99838 100%)'
      : 'linear-gradient(180deg, #5a5550 0%, #2c2620 45%, #14110d 55%, #322d27 100%)',
    boxShadow: selected
      ? 'inset 0 1px 0 rgba(255,240,180,0.8), inset 0 -1px 0 rgba(0,0,0,0.5), 0 0 0 1px #1a1410, 0 0 16px rgba(217,152,56,0.55), 0 4px 12px rgba(0,0,0,0.5)'
      : 'inset 0 1px 0 rgba(255,255,255,0.18), inset 0 -1px 0 rgba(0,0,0,0.6), 0 0 0 1px #0a0805, 0 4px 10px rgba(0,0,0,0.5)',
    color: selected ? '#1a1006' : '#f0e6c8',
    border: 'none',
    cursor: 'pointer',
    fontFamily: '"Trebuchet MS", "Lucida Sans", sans-serif',
    fontWeight: 'bold',
    textShadow: selected
      ? '0 1px 0 rgba(255,240,180,0.6)'
      : '0 -1px 0 rgba(0,0,0,0.7), 0 0 4px rgba(255,200,80,0.3)',
  });

  return (
    <FeatureShell feature="Memories">
      <SectionHead
        kicker="Memories · Home Video Vault"
        title="Watch the old DVDs"
        sub="The camcorder tapes Margot digitized in 2008. We render the menu the way the discs used to look — chunky chrome buttons, glossy chapter grid, that little flying logo. Click Play All to start the tape.">
        <NeuButton icon="film">Open vault</NeuButton>
        <NeuButton icon="cog">Settings</NeuButton>
      </SectionHead>

      {/* CRT housing */}
      <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center',
        minHeight: 0 }}>
        <div style={{
          width: '100%', maxWidth: 980,
          padding: 22, borderRadius: 26,
          background: 'linear-gradient(145deg, #cfc4b2, #8a7e6a)',
          boxShadow: '0 24px 50px -20px rgba(40,30,20,0.5), inset 0 1px 0 rgba(255,250,238,0.6), inset 0 -2px 0 rgba(60,45,30,0.3)',
          position: 'relative',
        }}>
          {/* TV brand label */}
          <div style={{ position: 'absolute', top: 8, left: '50%',
            transform: 'translateX(-50%)',
            fontFamily: '"Times New Roman", serif', fontStyle: 'italic',
            fontSize: 11, color: '#3a2f22', letterSpacing: 2, opacity: 0.7 }}>
            FAMILYTRON · 28″
          </div>

          {/* Power LED */}
          <div aria-hidden="true" style={{ position: 'absolute', bottom: 10, right: 24,
            width: 7, height: 7, borderRadius: '50%',
            background: '#d44a30',
            boxShadow: '0 0 6px #d44a30, inset 0 1px 0 rgba(255,200,180,0.6)' }}/>
          <div style={{ position: 'absolute', bottom: 8, right: 38,
            fontSize: 8, color: '#3a2f22', letterSpacing: 1, opacity: 0.6 }}>POWER</div>

          {/* Screen — keeps a 16:10 aspect inside the bezel */}
          <div style={{
            position: 'relative',
            aspectRatio: '16 / 10',
            background: '#000',
            borderRadius: 14,
            overflow: 'hidden',
            boxShadow: 'inset 0 0 0 4px #0a0805, inset 0 0 60px rgba(0,0,0,0.8)',
          }}>
            {/* Background loop — moving scene blurred behind menu */}
            <div style={{ position: 'absolute', inset: 0,
              transform: 'scale(1.1)', filter: 'blur(2px) brightness(0.55) saturate(1.2)' }}>
              <Photo scene="sunset" round={0}/>
            </div>
            <div style={{ position: 'absolute', inset: 0,
              background: 'radial-gradient(ellipse at center, rgba(0,0,0,0.1) 0%, rgba(0,0,0,0.7) 100%)' }}/>

            {/* Scanlines + slight curvature */}
            <div aria-hidden="true" style={{ position: 'absolute', inset: 0,
              background: 'repeating-linear-gradient(0deg, rgba(0,0,0,0.18) 0px, rgba(0,0,0,0.18) 1px, transparent 1px, transparent 3px)',
              pointerEvents: 'none', mixBlendMode: 'multiply' }}/>
            <div aria-hidden="true" style={{ position: 'absolute', inset: 0,
              background: 'radial-gradient(ellipse at 50% 45%, transparent 55%, rgba(0,0,0,0.5) 100%)',
              pointerEvents: 'none' }}/>

            {/* Menu content */}
            <div style={{ position: 'absolute', inset: 0,
              display: 'grid', gridTemplateColumns: '1fr 1.1fr',
              padding: '34px 36px', gap: 28,
              fontFamily: '"Trebuchet MS", "Lucida Sans", sans-serif' }}>

              {/* Left — title + main buttons */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 18,
                color: '#f0e6c8' }}>
                <div>
                  <div style={{ fontFamily: '"Times New Roman", serif',
                    fontStyle: 'italic', fontSize: 12, letterSpacing: 4,
                    color: '#d99838', textTransform: 'uppercase',
                    textShadow: '0 0 8px rgba(217,152,56,0.7), 0 1px 0 rgba(0,0,0,0.6)' }}>
                    The Alderhouse Family
                  </div>
                  <div style={{ fontFamily: '"Times New Roman", serif',
                    fontSize: 38, fontWeight: 'bold', lineHeight: 1.0,
                    marginTop: 6, letterSpacing: -0.5,
                    background: 'linear-gradient(180deg, #fff5d8 0%, #d99838 100%)',
                    WebkitBackgroundClip: 'text',
                    WebkitTextFillColor: 'transparent',
                    filter: 'drop-shadow(0 2px 0 rgba(0,0,0,0.5)) drop-shadow(0 0 12px rgba(217,152,56,0.4))' }}>
                    Summers at the Lake
                  </div>
                  <div style={{ fontSize: 11, color: '#d4c8a0', marginTop: 8,
                    letterSpacing: 1.5, fontStyle: 'italic' }}>
                    1999 — 2003 · Volume Two of Three
                  </div>
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginTop: 8 }}>
                  {[
                    { l: 'PLAY ALL',          sel: true,  caret: true },
                    { l: 'SCENE SELECTION',   sel: false },
                    { l: 'SPECIAL FEATURES',  sel: false },
                    { l: 'AUDIO & SUBTITLES', sel: false },
                  ].map((b, i) => (
                    <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                      <div style={{ width: 14, color: '#d99838',
                        textShadow: '0 0 6px rgba(217,152,56,0.7)',
                        fontSize: 14, opacity: b.sel ? 1 : 0 }}>▶</div>
                      <button style={{
                        ...chrome(b.sel),
                        flex: 1, padding: '10px 18px', borderRadius: 4,
                        fontSize: 13, letterSpacing: 2,
                        textAlign: 'left',
                      }}>{b.l}</button>
                    </div>
                  ))}
                </div>

                <div style={{ flex: 1 }}/>

                <div style={{ fontSize: 9.5, letterSpacing: 1.5,
                  color: '#8a7e60', fontStyle: 'italic' }}>
                  © 2003 ALDERHOUSE HOME PICTURES · DOLBY DIGITAL · NTSC · NOT FOR RESALE
                </div>
              </div>

              {/* Right — chapter grid */}
              <div style={{ display: 'flex', flexDirection: 'column' }}>
                <div style={{ fontFamily: '"Times New Roman", serif',
                  fontStyle: 'italic', fontSize: 11, letterSpacing: 3,
                  color: '#d99838', textTransform: 'uppercase',
                  marginBottom: 10,
                  textShadow: '0 0 6px rgba(217,152,56,0.6)' }}>
                  · Scene Selection ·
                </div>
                <div style={{ flex: 1, display: 'grid',
                  gridTemplateColumns: 'repeat(3, 1fr)',
                  gridTemplateRows: 'repeat(3, 1fr)', gap: 8 }}>
                  {chapters.map(c => (
                    <button key={c.n} aria-label={`Chapter ${c.n}: ${c.t}`}
                      style={{
                        position: 'relative', padding: 4, borderRadius: 3,
                        background: c.selected
                          ? 'linear-gradient(180deg, #ffe49a, #d99838)'
                          : 'linear-gradient(180deg, #2c2620, #14110d)',
                        boxShadow: c.selected
                          ? '0 0 0 1px #1a1006, 0 0 14px rgba(217,152,56,0.7), inset 0 1px 0 rgba(255,240,180,0.7)'
                          : '0 0 0 1px #0a0805, inset 0 1px 0 rgba(255,255,255,0.12)',
                        cursor: 'pointer', overflow: 'hidden',
                      }}>
                      <div style={{ position: 'relative', aspectRatio: '4 / 3',
                        overflow: 'hidden', borderRadius: 1 }}>
                        <Photo scene={c.scene} round={0} style={{ position: 'absolute', inset: 0,
                          filter: 'saturate(0.85) contrast(1.05) brightness(0.92)' }}/>
                        <div style={{ position: 'absolute', inset: 0,
                          background: 'linear-gradient(180deg, transparent 50%, rgba(0,0,0,0.7) 100%)' }}/>
                        {/* CH number */}
                        <div style={{ position: 'absolute', top: 4, left: 4,
                          padding: '1px 5px', borderRadius: 2,
                          background: 'rgba(0,0,0,0.7)',
                          fontSize: 9, fontWeight: 'bold', letterSpacing: 1,
                          color: c.selected ? '#ffd87a' : '#f0e6c8',
                          fontFamily: '"Trebuchet MS", sans-serif' }}>
                          CH {c.n.toString().padStart(2,'0')}
                        </div>
                        {/* Timecode */}
                        <div style={{ position: 'absolute', bottom: 3, right: 4,
                          fontFamily: '"Courier New", monospace',
                          fontSize: 9, color: '#f0e6c8',
                          textShadow: '0 1px 0 rgba(0,0,0,0.8)' }}>{c.tc}</div>
                        {/* Title */}
                        <div style={{ position: 'absolute', bottom: 3, left: 5,
                          right: 40,
                          fontSize: 9, color: '#f0e6c8', fontWeight: 'bold',
                          textShadow: '0 1px 0 rgba(0,0,0,0.8)',
                          whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                          {c.t}
                        </div>
                      </div>
                    </button>
                  ))}
                </div>

                {/* Pager dots */}
                <div style={{ display: 'flex', justifyContent: 'center',
                  gap: 6, marginTop: 12 }}>
                  {[1,2,3].map(i => (
                    <div key={i} style={{ width: 6, height: 6, borderRadius: 3,
                      background: i === 1 ? '#d99838' : 'rgba(217,152,56,0.25)',
                      boxShadow: i === 1 ? '0 0 6px #d99838' : 'none' }}/>
                  ))}
                </div>
              </div>
            </div>

            {/* Channel banner — top-right OSD */}
            <div aria-hidden="true" style={{ position: 'absolute', top: 14, right: 16,
              padding: '4px 10px',
              background: 'rgba(0,0,30,0.55)',
              border: '1px solid rgba(120,200,255,0.4)',
              fontFamily: '"Courier New", monospace',
              fontSize: 10, fontWeight: 'bold',
              color: '#a8e0ff', letterSpacing: 1.5,
              textShadow: '0 0 6px rgba(168,224,255,0.6)' }}>
              DVD · CH 04
            </div>

            {/* Bottom-right corner — DOLBY logo */}
            <div aria-hidden="true" style={{ position: 'absolute', bottom: 14, right: 16,
              padding: '2px 6px',
              border: '1px solid rgba(217,152,56,0.5)',
              fontFamily: 'sans-serif', fontSize: 8, fontWeight: 'bold',
              color: '#d4c8a0', letterSpacing: 1, opacity: 0.8 }}>
              DOLBY DIGITAL
            </div>
          </div>

          {/* Bezel buttons */}
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 16,
            marginTop: 14, paddingRight: 8 }}>
            {['POWER','MENU','VOL','CH'].map(b => (
              <div key={b} style={{ display: 'flex', flexDirection: 'column',
                alignItems: 'center', gap: 4 }}>
                <div style={{ width: 22, height: 22, borderRadius: '50%',
                  background: 'linear-gradient(145deg, #d8ccb8, #8a7e6a)',
                  boxShadow: 'inset 0 1px 0 rgba(255,250,238,0.6), inset 0 -1px 0 rgba(60,45,30,0.4), 0 2px 4px rgba(40,30,20,0.4)' }}/>
                <div style={{ fontSize: 8, color: '#3a2f22', letterSpacing: 1,
                  opacity: 0.7, fontFamily: 'sans-serif' }}>{b}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Lower rail — disc selector */}
      <div style={{ marginTop: 14, padding: 14, borderRadius: 16,
        background: CREAM, boxShadow: NEU_IN_SM,
        display: 'flex', alignItems: 'center', gap: 14 }}>
        <div style={{ fontSize: 10.5, fontWeight: 600, color: '#6e6358',
          textTransform: 'uppercase', letterSpacing: 1, flexShrink: 0 }}>
          Disc tray · 6
        </div>
        {[
          { y: '1996–98', t: 'Iris and the swing set', sel: false },
          { y: '1999–2003', t: 'Summers at the Lake', sel: true },
          { y: '2004–06', t: 'School plays', sel: false },
          { y: '2007–09', t: 'Road trips', sel: false },
          { y: '2010–12', t: 'The garden years', sel: false },
          { y: '2013–15', t: 'Theo, getting tall', sel: false },
        ].map((d, i) => (
          <button key={i} style={{
            display: 'flex', alignItems: 'center', gap: 8,
            padding: '6px 12px 6px 6px', borderRadius: 999,
            border: 'none', cursor: 'pointer',
            background: d.sel ? CREAM : CREAM,
            boxShadow: d.sel ? NEU_OUT_SM : NEU_IN_SM,
            color: d.sel ? '#b86348' : '#4a4035',
            fontFamily: SANS, fontSize: 11.5, fontWeight: 500,
            minHeight: 32,
          }}>
            <div style={{ width: 22, height: 22, borderRadius: '50%',
              background: 'radial-gradient(circle at 30% 30%, #f4ecdd 0%, #c0b6a0 30%, #6a5e4a 70%, #2a221a 100%)',
              boxShadow: 'inset 0 1px 1px rgba(255,250,238,0.4), 0 1px 2px rgba(40,30,20,0.3)',
              position: 'relative' }}>
              <div style={{ position: 'absolute', top: '50%', left: '50%',
                transform: 'translate(-50%,-50%)', width: 7, height: 7,
                borderRadius: '50%', background: '#1a1410',
                boxShadow: 'inset 0 0 2px rgba(0,0,0,0.6)' }}/>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column',
              alignItems: 'flex-start', textAlign: 'left' }}>
              <div style={{ fontSize: 9.5, color: '#8a7e70', letterSpacing: 0.8 }}>{d.y}</div>
              <div style={{ fontSize: 11, fontWeight: d.sel ? 600 : 500 }}>{d.t}</div>
            </div>
          </button>
        ))}
      </div>
    </FeatureShell>
  );
}

window.DvdMenu = DvdMenu;
