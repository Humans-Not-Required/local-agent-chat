import React, { useState, useEffect } from 'react';
import { styles } from '../styles';
import { API, senderColor, timeAgo, formatFullTimestamp } from '../utils';

export default function ParticipantPanel({ roomId, onClose }) {
  const [participants, setParticipants] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    fetch(`${API}/rooms/${roomId}/participants`)
      .then(r => r.ok ? r.json() : [])
      .then(data => { if (!cancelled) { setParticipants(data); setLoading(false); } })
      .catch(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [roomId]);

  return (
    <div className="participant-panel-wrapper" style={styles.participantPanel}>
      <div style={styles.participantHeader}>
        <span style={{ fontWeight: 600, fontSize: '0.9rem' }}>👥 Members ({participants.length})</span>
        <button onClick={onClose} style={styles.iconBtn}>✕</button>
      </div>
      <div style={styles.participantList}>
        {loading && <div style={{ padding: 16, color: '#64748b', fontSize: '0.85rem' }}>Loading...</div>}
        {!loading && participants.length === 0 && (
          <div style={{ padding: 16, color: '#64748b', fontSize: '0.85rem' }}>No messages yet</div>
        )}
        {participants.map(p => {
          const color = senderColor(p.sender);
          const typeIcon = p.sender_type === 'human' ? '👤' : p.sender_type === 'agent' ? '🤖' : '❓';
          const isRecent = (Date.now() - new Date(p.last_seen).getTime()) < 3600000;
          return (
            <div key={p.sender} style={styles.participantItem}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
                <div style={{
                  width: 8, height: 8, borderRadius: '50%', flexShrink: 0,
                  background: isRecent ? '#34d399' : '#475569',
                }} />
                <span style={{ fontSize: '1rem', flexShrink: 0 }}>{typeIcon}</span>
                <span style={{
                  fontWeight: 600, color, fontSize: '0.85rem',
                  overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                }}>
                  {p.sender}
                </span>
              </div>
              <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 2, paddingLeft: 26 }}>
                {p.message_count} msg{p.message_count !== 1 ? 's' : ''} · <span title={formatFullTimestamp(p.last_seen)}>{timeAgo(p.last_seen)}</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
