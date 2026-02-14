import React from 'react';

export default function ReactionChips({ reactions, sender, onToggle }) {
  if (!reactions || reactions.length === 0) return null;
  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginTop: 4 }}>
      {reactions.map(r => {
        const isMine = r.senders.includes(sender);
        return (
          <button
            key={r.emoji}
            onClick={(e) => { e.stopPropagation(); onToggle(r.emoji); }}
            title={r.senders.join(', ')}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 3,
              padding: '2px 6px',
              borderRadius: 10,
              border: isMine ? '1px solid #3b82f6' : '1px solid #334155',
              background: isMine ? 'rgba(59, 130, 246, 0.15)' : 'rgba(30, 41, 59, 0.6)',
              cursor: 'pointer',
              fontSize: '0.75rem',
              lineHeight: 1,
              color: '#e2e8f0',
              transition: 'all 0.15s',
            }}
            onMouseEnter={e => e.target.style.background = isMine ? 'rgba(59, 130, 246, 0.25)' : '#334155'}
            onMouseLeave={e => e.target.style.background = isMine ? 'rgba(59, 130, 246, 0.15)' : 'rgba(30, 41, 59, 0.6)'}
          >
            <span style={{ fontSize: '0.85rem' }}>{r.emoji}</span>
            <span>{r.count}</span>
          </button>
        );
      })}
    </div>
  );
}
