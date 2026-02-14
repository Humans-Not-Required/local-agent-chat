import React, { useEffect, useRef } from 'react';
import { senderColor } from '../utils';

const acStyles = {
  container: {
    position: 'absolute',
    bottom: '100%',
    left: 0,
    right: 0,
    marginBottom: 4,
    background: '#1e293b',
    border: '1px solid rgba(255,255,255,0.15)',
    borderRadius: 8,
    boxShadow: '0 -4px 16px rgba(0,0,0,0.4)',
    maxHeight: 200,
    overflowY: 'auto',
    zIndex: 40,
  },
  header: {
    padding: '6px 12px',
    fontSize: '0.7rem',
    color: '#64748b',
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
    borderBottom: '1px solid rgba(255,255,255,0.08)',
  },
  item: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '8px 12px',
    cursor: 'pointer',
    transition: 'background 0.1s',
  },
  itemActive: {
    background: 'rgba(255,255,255,0.1)',
  },
  avatar: {
    width: 24,
    height: 24,
    borderRadius: '50%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '0.7rem',
    fontWeight: 700,
    color: '#fff',
    flexShrink: 0,
  },
  name: {
    fontWeight: 600,
    fontSize: '0.85rem',
    color: '#f1f5f9',
  },
  type: {
    fontSize: '0.7rem',
    color: '#64748b',
    marginLeft: 'auto',
  },
};

// Participants are pre-filtered by ChatArea — this component just renders them
export default function MentionAutocomplete({ participants, activeIndex, onSelect }) {
  const listRef = useRef(null);

  // Scroll active item into view
  useEffect(() => {
    if (listRef.current && activeIndex >= 0) {
      const items = listRef.current.querySelectorAll('[data-mention-item]');
      if (items[activeIndex]) {
        items[activeIndex].scrollIntoView({ block: 'nearest' });
      }
    }
  }, [activeIndex]);

  return (
    <div style={acStyles.container} ref={listRef}>
      <div style={acStyles.header}>Mention someone</div>
      {participants.map((p, i) => (
        <div
          key={p.sender}
          data-mention-item
          style={{
            ...acStyles.item,
            ...(i === activeIndex ? acStyles.itemActive : {}),
          }}
          onMouseDown={(e) => {
            e.preventDefault(); // Prevent input blur
            onSelect(p.sender);
          }}
        >
          <div style={{ ...acStyles.avatar, background: senderColor(p.sender) }}>
            {p.sender.charAt(0).toUpperCase()}
          </div>
          <span style={acStyles.name}>@{p.sender}</span>
          <span style={acStyles.type}>
            {p.sender_type === 'agent' ? '🤖' : p.sender_type === 'human' ? '👤' : ''}
          </span>
        </div>
      ))}
    </div>
  );
}
