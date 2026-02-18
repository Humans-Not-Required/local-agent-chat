import React, { useState, useEffect, useMemo } from 'react';
import { styles } from '../styles';
import { API, senderColor, timeAgo, formatFullTimestamp, avatarFallbackUrl } from '../utils';

export default function ParticipantPanel({ roomId, onClose, onlineUsers }) {
  const [participants, setParticipants] = useState([]);
  const [loading, setLoading] = useState(true);
  const [expandedSender, setExpandedSender] = useState(null);

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
          const effectiveAvatarUrl = p.avatar_url || avatarFallbackUrl(p.sender, 56);
          const hasProfile = p.display_name || effectiveAvatarUrl || p.bio || p.status_text;
          const isExpanded = expandedSender === p.sender;
          const displayName = p.display_name || p.sender;

          return (
            <div key={p.sender}
              style={{
                ...styles.participantItem,
                cursor: hasProfile ? 'pointer' : 'default',
                background: isExpanded ? 'rgba(99, 102, 241, 0.08)' : 'transparent',
                borderRadius: 8,
                transition: 'background 0.15s',
              }}
              onClick={() => hasProfile && setExpandedSender(isExpanded ? null : p.sender)}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
                {/* Avatar or online dot */}
                {effectiveAvatarUrl ? (
                  <div style={{ position: 'relative', flexShrink: 0 }}>
                    <img
                      src={effectiveAvatarUrl}
                      alt={displayName}
                      style={{
                        width: 28, height: 28, borderRadius: '50%',
                        objectFit: 'cover', border: `2px solid ${isOnline ? '#34d399' : '#334155'}`,
                      }}
                      onError={e => { e.target.style.display = 'none'; }}
                    />
                    {isOnline && (
                      <div style={{
                        position: 'absolute', bottom: -1, right: -1,
                        width: 10, height: 10, borderRadius: '50%',
                        background: '#34d399', border: '2px solid #1e293b',
                      }} />
                    )}
                  </div>
                ) : (
                  <div style={{
                    width: 8, height: 8, borderRadius: '50%', flexShrink: 0,
                    background: isOnline ? '#34d399' : '#475569',
                    boxShadow: isOnline ? '0 0 6px rgba(52, 211, 153, 0.5)' : 'none',
                  }} />
                )}
                <span style={{ fontSize: '1rem', flexShrink: 0 }}>{typeIcon}</span>
                <div style={{ minWidth: 0, overflow: 'hidden' }}>
                  <span style={{
                    fontWeight: 600, color, fontSize: '0.85rem',
                    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                    display: 'block',
                  }}>
                    {displayName}
                  </span>
                  {p.display_name && p.display_name !== p.sender && (
                    <span style={{ fontSize: '0.7rem', color: '#64748b' }}>@{p.sender}</span>
                  )}
                </div>
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
                {p.status_text && !isExpanded && (
                  <span style={{
                    fontSize: '0.65rem', color: '#94a3b8',
                    background: 'rgba(148, 163, 184, 0.1)',
                    padding: '1px 6px', borderRadius: 8,
                    fontWeight: 400, flexShrink: 0,
                    maxWidth: 80, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                  }}>
                    {p.status_text}
                  </span>
                )}
              </div>
              <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 2, paddingLeft: effectiveAvatarUrl ? 38 : 26 }}>
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
              {/* Expanded profile card */}
              {isExpanded && hasProfile && (
                <div style={{
                  marginTop: 8, paddingLeft: effectiveAvatarUrl ? 38 : 26,
                  borderTop: '1px solid rgba(148, 163, 184, 0.15)',
                  paddingTop: 8,
                }}>
                  {p.bio && (
                    <div style={{ fontSize: '0.8rem', color: '#cbd5e1', lineHeight: 1.4, marginBottom: 4 }}>
                      {p.bio}
                    </div>
                  )}
                  {p.status_text && (
                    <div style={{ fontSize: '0.75rem', color: '#94a3b8', marginBottom: 2 }}>
                      💬 {p.status_text}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
