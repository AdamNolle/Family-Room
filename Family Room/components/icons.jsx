// Icon kit — single-path SVGs sized 1em, stroke 1.6, no emoji.
// Usage: <Icon name="home" size={20} color="#2a221a" />

function Icon({ name, size = 20, color = 'currentColor', strokeWidth = 1.6, style = {} }) {
  const props = { width: size, height: size, viewBox: '0 0 24 24', fill: 'none',
    stroke: color, strokeWidth, strokeLinecap: 'round', strokeLinejoin: 'round',
    style, 'aria-hidden': true, focusable: false };
  switch (name) {
    case 'home':
      return <svg {...props}><path d="M3 11 L12 4 L21 11 V20 a1 1 0 0 1 -1 1 H4 a1 1 0 0 1 -1 -1 Z"/><path d="M9 21 V14 H15 V21"/></svg>;
    case 'people':
      return <svg {...props}><circle cx="9" cy="8" r="3.5"/><path d="M3 19 c1-3 3.5-4.5 6-4.5 s5 1.5 6 4.5"/><circle cx="17" cy="9" r="2.8"/><path d="M21 18 c-.6-2.2-2.2-3.3-4-3.3"/></svg>;
    case 'plus':
      return <svg {...props}><path d="M12 5 V19 M5 12 H19"/></svg>;
    case 'clock':
      return <svg {...props}><circle cx="12" cy="12" r="8.5"/><path d="M12 7 V12 L15.5 14"/></svg>;
    case 'cog':
      return <svg {...props}><circle cx="12" cy="12" r="3"/><path d="M12 2 V5 M12 19 V22 M22 12 H19 M5 12 H2 M19 5 L17 7 M7 17 L5 19 M19 19 L17 17 M7 7 L5 5"/></svg>;
    case 'search':
      return <svg {...props}><circle cx="11" cy="11" r="6.5"/><path d="M16 16 L21 21"/></svg>;
    case 'album':
      return <svg {...props}><rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M3 15 L8 11 L13 14 L18 9 L21 11"/><circle cx="8" cy="9" r="1.4"/></svg>;
    case 'film':
      return <svg {...props}><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 4 V20 M17 4 V20"/><path d="M3 9 H7 M17 9 H21 M3 15 H7 M17 15 H21 M3 12 H21"/></svg>;
    case 'play':
      return <svg {...props} fill={color}><path d="M7 5 L19 12 L7 19 Z" stroke="none"/></svg>;
    case 'pause':
      return <svg {...props} fill={color}><rect x="6" y="5" width="4" height="14" rx="1" stroke="none"/><rect x="14" y="5" width="4" height="14" rx="1" stroke="none"/></svg>;
    case 'wand':
      return <svg {...props}><path d="M5 19 L18 6"/><path d="M16 4 L18 6 L20 8"/><path d="M3 7 L4.5 8 M3 11 L4.5 10 M11 3 L12 4.5 M15 3 L14 4.5"/></svg>;
    case 'tag':
      return <svg {...props}><path d="M3 13 V4 H12 L21 13 L13 21 Z"/><circle cx="8" cy="8" r="1.4"/></svg>;
    case 'merge':
      return <svg {...props}><path d="M6 4 V10 a3 3 0 0 0 3 3 H15 a3 3 0 0 1 3 3 V20"/><path d="M3 7 L6 4 L9 7 M15 17 L18 20 L21 17"/></svg>;
    case 'check':
      return <svg {...props}><path d="M5 12 L10 17 L20 7"/></svg>;
    case 'sparkles':
      return <svg {...props}><path d="M12 4 L13.5 9 L18 10.5 L13.5 12 L12 17 L10.5 12 L6 10.5 L10.5 9 Z"/><path d="M19 16 L19.7 17.5 L21 18 L19.7 18.5 L19 20 L18.3 18.5 L17 18 L18.3 17.5 Z"/></svg>;
    case 'family':
      return <svg {...props}><circle cx="7" cy="7" r="2.5"/><circle cx="17" cy="7" r="2.5"/><circle cx="12" cy="16" r="2.5"/><path d="M3 21 c.5-2 2-3 4-3 M21 21 c-.5-2-2-3-4-3 M8 21 c.5-1.5 2-2.2 4-2.2 s3.5.7 4 2.2"/></svg>;
    case 'friends':
      return <svg {...props}><circle cx="8" cy="9" r="3"/><circle cx="16" cy="9" r="3"/><path d="M3 19 c.8-3 2.8-4.5 5-4.5 M21 19 c-.8-3-2.8-4.5-5-4.5"/><path d="M9 19 c.5-1.5 1.7-2.2 3-2.2 s2.5.7 3 2.2"/></svg>;
    case 'invite':
      return <svg {...props}><path d="M3 6 H21 V18 H3 Z"/><path d="M3 7 L12 13 L21 7"/></svg>;
    case 'dot':
      return <svg {...props} fill={color}><circle cx="12" cy="12" r="3" stroke="none"/></svg>;
    case 'chevron-right':
      return <svg {...props}><path d="M9 5 L16 12 L9 19"/></svg>;
    case 'chevron-left':
      return <svg {...props}><path d="M15 5 L8 12 L15 19"/></svg>;
    case 'chevron-down':
      return <svg {...props}><path d="M5 9 L12 16 L19 9"/></svg>;
    case 'more':
      return <svg {...props} fill={color}><circle cx="6" cy="12" r="1.6" stroke="none"/><circle cx="12" cy="12" r="1.6" stroke="none"/><circle cx="18" cy="12" r="1.6" stroke="none"/></svg>;
    case 'heart':
      return <svg {...props}><path d="M12 20 C 7 16, 3 13, 3 9 a4 4 0 0 1 8 -1 a4 4 0 0 1 9 1 c0 4 -4 7 -8 11 Z"/></svg>;
    case 'comment':
      return <svg {...props}><path d="M4 5 H20 V16 H13 L8 20 V16 H4 Z"/></svg>;
    case 'share':
      return <svg {...props}><circle cx="6" cy="12" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M8 11 L16 7 M8 13 L16 17"/></svg>;
    case 'pin':
      return <svg {...props}><path d="M12 21 c-4-5-7-8-7-12 a7 7 0 0 1 14 0 c0 4-3 7-7 12 Z"/><circle cx="12" cy="9" r="2.5"/></svg>;
    case 'photo':
      return <svg {...props}><rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="M3 17 L9 12 L13 15 L17 11 L21 14"/></svg>;
    case 'video':
      return <svg {...props}><rect x="3" y="6" width="13" height="12" rx="2"/><path d="M16 10 L21 7 V17 L16 14 Z"/></svg>;
    case 'scissors':
      return <svg {...props}><circle cx="6" cy="7" r="2.5"/><circle cx="6" cy="17" r="2.5"/><path d="M8 9 L20 18 M8 15 L20 6"/></svg>;
    case 'music':
      return <svg {...props}><path d="M9 18 V6 L20 4 V16"/><circle cx="7" cy="18" r="2"/><circle cx="18" cy="16" r="2"/></svg>;
    case 'text':
      return <svg {...props}><path d="M5 7 V5 H19 V7 M12 5 V19 M9 19 H15"/></svg>;
    case 'transition':
      return <svg {...props}><rect x="3" y="6" width="7" height="12" rx="1"/><rect x="14" y="6" width="7" height="12" rx="1"/><path d="M11 12 H13"/></svg>;
    case 'volume':
      return <svg {...props}><path d="M3 10 V14 H6 L11 18 V6 L6 10 Z"/><path d="M14 9 c1.5 1.5 1.5 4.5 0 6 M17 7 c3 3 3 7 0 10"/></svg>;
    case 'fullscreen':
      return <svg {...props}><path d="M4 9 V4 H9 M15 4 H20 V9 M20 15 V20 H15 M9 20 H4 V15"/></svg>;
    case 'role-admin':
      return <svg {...props}><path d="M12 3 L20 7 V12 c0 5-4 8-8 9 -4-1-8-4-8-9 V7 Z"/><path d="M9 12 L11 14 L15 10"/></svg>;
    case 'logout':
      return <svg {...props}><path d="M14 4 H5 V20 H14"/><path d="M10 12 H21 M17 8 L21 12 L17 16"/></svg>;
    default:
      return <svg {...props}><circle cx="12" cy="12" r="8"/></svg>;
  }
}

window.Icon = Icon;
