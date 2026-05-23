// Shared neumorphic primitives + Mac frame (re-exported MacChrome but reusable for new screens).
// Uses globals from familyroom.jsx (PEOPLE, Avatar, Photo, NEU_*, CREAM, SANS, DISPLAY).

function NeuButton({ children, primary = false, size = 'md', icon, onClick, ariaLabel, style = {} }) {
  const pad = size === 'sm' ? '8px 12px' : size === 'lg' ? '14px 22px' : '11px 16px';
  const fs = size === 'sm' ? 12 : size === 'lg' ? 14 : 13;
  return (
    <button
      onClick={onClick}
      aria-label={ariaLabel}
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 8,
        padding: pad, borderRadius: 12, border: 'none',
        background: CREAM,
        boxShadow: NEU_OUT_SM,
        color: primary ? '#b86348' : '#2a221a',
        fontWeight: primary ? 600 : 500,
        fontSize: fs, fontFamily: SANS,
        cursor: 'pointer',
        minHeight: 36,
        ...style,
      }}>
      {icon && <Icon name={icon} size={fs + 4} color={primary ? '#b86348' : '#4a4035'}/>}
      {children}
    </button>
  );
}

function NeuCard({ children, inset = false, padding = 16, style = {} }) {
  return (
    <div style={{
      background: CREAM, borderRadius: 18, padding,
      boxShadow: inset ? NEU_IN_SM : NEU_OUT_SM,
      ...style,
    }}>{children}</div>
  );
}

function NeuChip({ children, active = false, icon, onClick, style = {} }) {
  return (
    <button
      onClick={onClick}
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 6,
        padding: '7px 12px', borderRadius: 999, border: 'none',
        background: CREAM,
        boxShadow: active ? NEU_IN_SM : NEU_OUT_SM,
        color: active ? '#b86348' : '#4a4035',
        fontWeight: active ? 600 : 500, fontSize: 12, fontFamily: SANS,
        cursor: 'pointer', minHeight: 32,
        ...style,
      }}>
      {icon && <Icon name={icon} size={14} color={active ? '#b86348' : '#4a4035'}/>}
      {children}
    </button>
  );
}

// Section heading inside a Mac canvas
function SectionHead({ kicker, title, sub, children }) {
  return (
    <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between',
      gap: 24, marginBottom: 18 }}>
      <div style={{ minWidth: 0 }}>
        {kicker && (
          <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: 1.2,
            textTransform: 'uppercase', color: '#b86348' }}>{kicker}</div>
        )}
        <div style={{ fontFamily: DISPLAY, fontSize: 30, fontWeight: 500,
          color: '#2a221a', letterSpacing: -0.6, marginTop: 4, lineHeight: 1.05 }}>{title}</div>
        {sub && (
          <div style={{ fontSize: 13.5, color: '#4a4035', marginTop: 6, maxWidth: 580, lineHeight: 1.5 }}>{sub}</div>
        )}
      </div>
      {children && <div style={{ display: 'flex', gap: 8, flexShrink: 0 }}>{children}</div>}
    </div>
  );
}

// A small label + value pair used in stat strips
function StatBlock({ label, value, hint }) {
  return (
    <div style={{ flex: 1, padding: '14px 16px', borderRadius: 14,
      background: CREAM, boxShadow: NEU_IN_SM }}>
      <div style={{ fontFamily: DISPLAY, fontSize: 24, fontWeight: 500, color: '#2a221a', letterSpacing: -0.4 }}>{value}</div>
      <div style={{ fontSize: 11, fontWeight: 600, color: '#6e6358', textTransform: 'uppercase', letterSpacing: 0.6, marginTop: 2 }}>{label}</div>
      {hint && <div style={{ fontSize: 11, color: '#8a7e70', marginTop: 4 }}>{hint}</div>}
    </div>
  );
}

// Standardized Mac shell for feature pages — left rail with feature nav, right canvas
function FeatureShell({ feature, family, children, width = 1280, height = 820 }) {
  const featureIcons = {
    'Home':       'home',
    'People':     'people',
    'Albums':     'album',
    'Memories':   'film',
    'Editor':     'scissors',
    'Settings':   'cog',
  };
  const navItems = ['Home','People','Albums','Memories','Editor','Settings'];

  return (
    <MacChrome title={`Family Room — ${feature}`} width={width} height={height}>
      <div style={{ flex: 1, display: 'flex', minHeight: 0, background: CREAM, padding: 18, gap: 18 }}>
        {/* Slim icon rail */}
        <nav aria-label="App navigation" style={{ width: 72, flexShrink: 0,
          padding: '14px 8px', borderRadius: 22, background: CREAM, boxShadow: NEU_IN,
          display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
          {/* Family avatar / switcher */}
          <button aria-label={`Active family: ${family || 'The Alderhouse'}`} style={{
            width: 44, height: 44, borderRadius: 14, border: 'none',
            background: 'linear-gradient(135deg, #f0bfa8, #c97a5d)',
            color: '#fff', fontFamily: DISPLAY, fontSize: 18, fontWeight: 600,
            cursor: 'pointer', boxShadow: NEU_OUT_SM, marginBottom: 8,
          }}>A</button>

          {navItems.map(n => {
            const on = n === feature;
            return (
              <button key={n}
                aria-label={n}
                aria-current={on ? 'page' : undefined}
                style={{
                  width: 44, height: 44, borderRadius: 12, border: 'none',
                  background: CREAM,
                  boxShadow: on ? NEU_IN_SM : 'none',
                  cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>
                <Icon name={featureIcons[n]} size={20} color={on ? '#b86348' : '#4a4035'}/>
              </button>
            );
          })}

          <div style={{ flex: 1 }}/>

          <button aria-label="Profile" style={{ width: 40, height: 40, borderRadius: 20,
            border: 'none', padding: 0, cursor: 'pointer', boxShadow: NEU_OUT_SM,
            background: CREAM }}>
            <Avatar p={PEOPLE[2]} size={36}/>
          </button>
        </nav>

        {/* Right canvas */}
        <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column',
          borderRadius: 22, background: CREAM, padding: 24, overflow: 'hidden' }}>
          {children}
        </div>
      </div>
    </MacChrome>
  );
}

window.NeuButton = NeuButton;
window.NeuCard = NeuCard;
window.NeuChip = NeuChip;
window.SectionHead = SectionHead;
window.StatBlock = StatBlock;
window.FeatureShell = FeatureShell;
