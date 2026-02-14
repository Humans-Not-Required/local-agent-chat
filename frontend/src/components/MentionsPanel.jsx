import React, { useState, useEffect, useCallback, useRef } from 'react';
import { styles } from '../styles';
import { API, timeAgo, formatFullTimestamp, senderColor } from '../utils';

export default function MentionsPanel({ sender, onClose, rooms, onSelectRoom }) {
  const [mentions, setMentions] = useState([]);
  const [unreadCount, setUnreadCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const pollRef = useRef(null);

  const fetchMentions = useCallback(async () => {
    if (!sender) return;
    try {
      const res = await fetch(`${API}/mentions?target=${encodeURIComponent(sender)}&limit=50`);
      if (res.ok) {
        const data = await res.json();
        setMentions(data.mentions || []);
      }
    } catch (e) {
      setError('Failed to load mentions');
    }
  }, [sender]);

  const fetchUnread = useCallback(async () => {
    if (!sender) return;
    try {
      const res = await fetch(`${API}/mentions/unread?target=${encodeURIComponent(sender)}`);
      if (res.ok) {
        const data = await res.json();
        setUnreadCount(data.total_unread || 0);
      }
    } catch (e) { /* ignore */ }
  }, [sender]);

  useEffect(() => {
    setLoading(true);
    Promise.all([fetchMentions(), fetchUnread()]).finally(() => setLoading(false));
    // Poll every 30 seconds
    pollRef.current = setInterval(() => {
      fetchMentions();
      fetchUnread();
    }, 30000);
    return () => clearInterval(pollRef.current);
  }, [fetchMentions, fetchUnread]);

  const handleMentionClick = (mention) => {
    const room = rooms.find(r => r.id === mention.room_id);
    if (room) {
      onSelectRoom(room);
      onClose();
    }
  };

  const highlightMention = (content) => {
    if (!content || !sender) return content;
    const regex = new RegExp(`(@${sender.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    const parts = content.split(regex);
    return parts.map((part, i) =>
      regex.test(part)
        ? React.createElement('mark', {
            key: i,
            style: { background: '#7c3aed', color: '#fff', borderRadius: 2, padding: '0 2px' }
          }, part)
        : part
    );
  };

  return (
    <div style={styles.searchPanel}>
      <div style={styles.searchHeader}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}>
          <span style={{ fontSize: '1rem' }}>🔔</span>
          <span style={{ color: '#e2e8f0', fontWeight: 600, fontSize: '0.95rem' }}>
            Mentions
          </span>
          {unreadCount > 0 && (
            <span style={{
              background: '#7c3aed',
              color: '#fff',
              fontSize: '0.7rem',
              fontWeight: 700,
              padding: '1px 6px',
              borderRadius: 10,
              minWidth: 18,
              textAlign: 'center'
            }}>
              {unreadCount} unread
            </span>
          )}
        </div>
        <button onClick={onClose} style={styles.searchCloseBtn}>Close</button>
      </div>
      <div style={styles.searchResults}>
        {loading && (
          <div style={{ textAlign: 'center', padding: 20, color: '#64748b' }}>Loading mentions...</div>
        )}
        {!loading && error && (
          <div style={{ textAlign: 'center', padding: 20, color: '#ef4444' }}>{error}</div>
        )}
        {!loading && !error && mentions.length === 0 && (
          <div style={{ textAlign: 'center', padding: 40, color: '#64748b' }}>
            <div style={{ fontSize: '1.5rem', marginBottom: 8 }}>🔔</div>
            <div style={{ fontSize: '0.9rem' }}>No mentions yet</div>
            <div style={{ fontSize: '0.8rem', marginTop: 4, color: '#475569' }}>
              When someone @mentions you, it'll show up here
            </div>
          </div>
        )}
        {!loading && !error && mentions.map(m => (
          <div
            key={m.message_id}
            onClick={() => handleMentionClick(m)}
            style={{
              ...styles.searchResultItem,
              cursor: 'pointer',
              borderLeft: '3px solid #7c3aed',
              paddingLeft: 12
            }}
            onMouseEnter={e => e.currentTarget.style.background = '#1e293b'}
            onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
          >
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ fontSize: '0.75rem', color: '#7c3aed', fontWeight: 600 }}>
                  #{m.room_name || 'unknown'}
                </span>
                <span style={{ fontSize: '0.8rem', fontWeight: 600, color: senderColor(m.sender) }}>
                  {m.sender_type === 'human' ? '👤' : '🤖'} {m.sender}
                </span>
              </div>
              <span
                style={{ fontSize: '0.7rem', color: '#475569' }}
                title={formatFullTimestamp(m.created_at)}
              >
                {timeAgo(m.created_at)}
              </span>
            </div>
            <div style={{
              fontSize: '0.85rem',
              color: '#cbd5e1',
              lineHeight: 1.4,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              display: '-webkit-box',
              WebkitLineClamp: 2,
              WebkitBoxOrient: 'vertical'
            }}>
              {highlightMention(m.content)}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
