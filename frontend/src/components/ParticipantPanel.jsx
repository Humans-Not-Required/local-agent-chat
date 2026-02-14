import React, { useState, useEffect, useMemo } from 'react';
import { styles } from '../styles';
import { API, senderColor, timeAgo, formatFullTimestamp } from '../utils';

export default function ParticipantPanel({ roomId, onClose, onlineUsers }) {
  const [participants, setParticipants] = useState([]);
  const [loading, setLoading] = useState(true);

  // Build a set of online sender names for O(1) lookup
  const onlineSet = useMemo(() => {
    const set = new Set();
    if (onlineUsers) {
      for (const u of onlineUsers) {
        set.add(u.sender);
      }
    }
    return set;
  }, [onlineUsers]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    fetch(`${API}/rooms/${roomId}/participants`)
      .then(r => r.ok ? r.json() : [])
      .then(data => { if (!cancelled) { setParticipants(data); setLoading(false); } })
      .catch(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [roomId]);

  // Merge online-only users who haven't sent messages yet
  const allMembers = useMemo(() => {
    const participantSet = new Set(participants.map(p => p.sender));
    const onlineOnly = (onlineUsers || [])
      .filter(u => !participantSet.has(u.sender))
      .map(u => ({
        sender: u.sender,
        sender_type: u.sender_type,
        message_count: 0,
        first_seen: u.connected_at,
        last_seen: u.connected_at,
      }));
    // Sort: online first, then by last_seen
    const merged = [...participants, ...onlineOnly];
    merged.sort((a, b) => {
      const aOnline = onlineSet.has(a.sender) ? 0 : 1;
      const bOnline = onlineSet.has(b.sender) ? 0 : 1;
      if (aOnline !== bOnline) return aOnline - bOnline;
      return new Date(b.last_seen) - new Date(a.last_seen);
    });
    return merged;
  }, [participants, onlineUsers, onlineSet]);

  const onlineCount = onlineUsers?.length || 0;

  return (
    <div className="participant-panel-wrapper" style={styles.participantPanel}>
      <div style={styles.participantHeader}>
        <span style={{ fontWeight: 600, fontSize: '0.9rem' }}>
          👥 Members ({allMembers.length})
          {onlineCount > 0 && (
            <span style={{ color: '#34d399', fontWeight: 400, fontSize: '0.8rem', marginLeft: 6 }}>
              {onlineCount} online
            </span>
          )}
        </span>
        <button onClick={onClose} style={styles.iconBtn}>✕</button>
      </div>
      <div style={styles.participantList}>
        {loading && <div style={{ padding: 16, color: '#64748b', fontSize: '0.85rem' }}>Loading...</div>}
        {!loading && allMembers.length === 0 && (
          <div style={{ padding: 16, color: '#64748b', fontSize: '0.85rem' }}>No members yet</div>
        )}
        {allMembers.map(p => {
          const color = senderColor(p.sender);
          const typeIcon = p.sender_type === 'human' ? '👤' : p.sender_type === 'agent' ? '🤖' : '❓';
          const isOnline = onlineSet.has(p.sender);
          return (
            <div key={p.sender} style={styles.participantItem}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
                <div style={{
                  width: 8, height: 8, borderRadius: '50%', flexShrink: 0,
                  background: isOnline ? '#34d399' : '#475569',
                  boxShadow: isOnline ? '0 0 6px rgba(52, 211, 153, 0.5)' : 'none',
                }} />
                <span style={{ fontSize: '1rem', flexShrink: 0 }}>{typeIcon}</span>
                <span style={{
                  fontWeight: 600, color, fontSize: '0.85rem',
                  overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                }}>
                  {p.sender}
                </span>
                {isOnline && (
                  <span style={{
                    fontSize: '0.65rem', color: '#34d399',
                    background: 'rgba(52, 211, 153, 0.1)',
                    padding: '1px 6px', borderRadius: 8,
                    fontWeight: 500, flexShrink: 0,
                  }}>
                    online
                  </span>
                )}
              </div>
              <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 2, paddingLeft: 26 }}>
                {p.message_count > 0 ? (
                  <>
                    {p.message_count} msg{p.message_count !== 1 ? 's' : ''} · <span title={formatFullTimestamp(p.last_seen)}>{timeAgo(p.last_seen)}</span>
                  </>
                ) : isOnline ? (
                  <span style={{ color: '#34d399' }}>Connected</span>
                ) : (
                  <span>No messages</span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
