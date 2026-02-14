import React, { useState } from 'react';
import { styles } from '../styles';
import { timeAgo, formatFullTimestamp, senderColor } from '../utils';
import ChatLogo from './ChatLogo';

export default function RoomList({ rooms, activeRoom, onSelect, onCreateRoom, unreadCounts, sender, senderType, onChangeSender, onEditProfile }) {
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const [newDesc, setNewDesc] = useState('');

  const handleCreate = async (e) => {
    e.preventDefault();
    if (!newName.trim()) return;
    await onCreateRoom(newName.trim(), newDesc.trim());
    setNewName('');
    setNewDesc('');
    setCreating(false);
  };

  return (
    <div className="chat-sidebar" style={styles.sidebar}>
      <div style={styles.sidebarHeader}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <ChatLogo size={22} />
          <h2 style={{ fontSize: '1rem', fontWeight: 600 }}>Rooms</h2>
        </div>
        <button onClick={() => setCreating(!creating)} style={styles.iconBtn} title="Create room">+</button>
      </div>
      {creating && (
        <form onSubmit={handleCreate} style={styles.createForm}>
          <input
            value={newName}
            onChange={e => setNewName(e.target.value)}
            placeholder="Room name"
            style={styles.input}
            autoFocus
          />
          <input
            value={newDesc}
            onChange={e => setNewDesc(e.target.value)}
            placeholder="Description (optional)"
            style={styles.input}
          />
          <div style={{ display: 'flex', gap: 6 }}>
            <button type="submit" style={styles.btnPrimary}>Create</button>
            <button type="button" onClick={() => setCreating(false)} style={styles.btnSecondary}>Cancel</button>
          </div>
        </form>
      )}
      <div style={styles.roomList}>
        {rooms.map(room => {
          const unread = unreadCounts[room.id] || 0;
          return (
            <div
              key={room.id}
              onClick={() => onSelect(room)}
              style={{
                ...styles.roomItem,
                background: activeRoom?.id === room.id ? '#1e293b' : 'transparent',
                borderLeft: activeRoom?.id === room.id ? '3px solid #3b82f6' : '3px solid transparent',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <div style={{ fontWeight: unread > 0 ? 700 : 500, color: activeRoom?.id === room.id ? '#f1f5f9' : unread > 0 ? '#f1f5f9' : '#cbd5e1' }}>
                  #{room.name}
                </div>
                {unread > 0 && (
                  <span style={styles.unreadBadge}>
                    {unread > 99 ? '99+' : unread}
                  </span>
                )}
              </div>
              {room.last_message_sender && room.last_message_preview && (
                <div style={{ fontSize: '0.75rem', color: '#94a3b8', marginTop: 2, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  <span style={{ color: '#cbd5e1', fontWeight: 500 }}>{room.last_message_sender}:</span>{' '}
                  {room.last_message_preview.length > 60 ? room.last_message_preview.slice(0, 60) + '…' : room.last_message_preview}
                </div>
              )}
              <div style={{ fontSize: '0.7rem', color: '#64748b', marginTop: 2 }}>
                {room.message_count || 0} msgs
                {room.last_activity && <span title={formatFullTimestamp(room.last_activity)}>{` · ${timeAgo(room.last_activity)}`}</span>}
              </div>
            </div>
          );
        })}
        {rooms.length === 0 && (
          <div style={{ padding: '16px', color: '#64748b', fontSize: '0.85rem' }}>No rooms yet</div>
        )}
      </div>
      {/* User identity footer */}
      {sender && (
        <div style={styles.sidebarFooter}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, minWidth: 0 }}>
            <span style={{ fontSize: '0.9rem', flexShrink: 0 }}>{senderType === 'human' ? '👤' : '🤖'}</span>
            <span style={{
              fontSize: '0.8rem',
              color: senderColor(sender),
              fontWeight: 600,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}>
              {sender}
            </span>
          </div>
          <button
            onClick={onEditProfile}
            style={{ ...styles.iconBtn, fontSize: '0.75rem', padding: '2px 8px', border: 'none' }}
            title="Edit profile"
          >
            👤
          </button>
          <button
            onClick={onChangeSender}
            style={{ ...styles.iconBtn, fontSize: '0.75rem', padding: '2px 8px', border: 'none' }}
            title="Change name"
          >
            ✎
          </button>
        </div>
      )}
    </div>
  );
}
