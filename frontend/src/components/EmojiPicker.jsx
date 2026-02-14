import React, { useRef, useEffect } from 'react';

const QUICK_EMOJIS = ['👍', '❤️', '😂', '🎉', '🤔', '👀', '🔥', '✅', '❌', '🙌', '💯', '🚀'];

export default function EmojiPicker({ onSelect, onClose }) {
  const ref = useRef(null);
  useEffect(() => {
    const handler = (e) => {
      if (ref.current && !ref.current.contains(e.target)) onClose();
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [onClose]);

  return (
    <div ref={ref} style={{
      position: 'absolute',
      bottom: '100%',
      right: 0,
      marginBottom: 4,
      background: '#1e293b',
      border: '1px solid #334155',
      borderRadius: 8,
      padding: 6,
      display: 'grid',
      gridTemplateColumns: 'repeat(6, 1fr)',
      gap: 2,
      zIndex: 20,
      boxShadow: '0 4px 12px rgba(0,0,0,0.4)',
    }}>
      {QUICK_EMOJIS.map(emoji => (
        <button
          key={emoji}
          onClick={(e) => { e.stopPropagation(); onSelect(emoji); }}
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            fontSize: '1.1rem',
            padding: 4,
            borderRadius: 4,
            lineHeight: 1,
          }}
          onMouseEnter={e => e.target.style.background = '#334155'}
          onMouseLeave={e => e.target.style.background = 'none'}
        >
          {emoji}
        </button>
      ))}
    </div>
  );
}
