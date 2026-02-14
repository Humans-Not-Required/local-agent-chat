import React from 'react';
import { styles } from '../styles';
import ChatLogo from './ChatLogo';

export default function ChatHeader({
  room, connected,
  showMentions, onToggleMentions, mentionCount,
  onToggleSearch, showSearch,
  showPins, onTogglePins,
  showParticipants, onToggleParticipants, onlineUsers,
  onOpenSettings, showSettings,
  soundEnabled, onToggleSound,
}) {
  return (
    <div className="chat-content-header" style={styles.chatHeader}>
      <div className="chat-room-info" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <ChatLogo size={20} />
        <span style={{ fontWeight: 600, fontSize: '1rem' }}>#{room.name}</span>
        {room.description && (
          <span style={{ color: '#64748b', fontSize: '0.85rem' }}>{room.description}</span>
        )}
      </div>
      <div className="chat-header-actions" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <div className="chat-live-indicator" style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <div style={{
            width: 8, height: 8, borderRadius: '50%',
            background: connected ? '#34d399' : '#ef4444',
          }} />
          <span className="chat-live-text" style={{ fontSize: '0.75rem', color: '#64748b' }}>
            {connected ? 'Live' : 'Reconnecting...'}
          </span>
        </div>
        <button
          onClick={onToggleMentions}
          style={{
            ...styles.iconBtn,
            background: showMentions ? '#334155' : 'none',
            fontSize: '0.9rem',
            padding: '4px 8px',
            position: 'relative',
          }}
          title={`Mentions${mentionCount > 0 ? ` (${mentionCount} unread)` : ''}`}
        >
          @
          {mentionCount > 0 && (
            <span style={{
              position: 'absolute', top: -2, right: -2,
              background: '#7c3aed', color: '#fff',
              fontSize: '0.55rem', fontWeight: 700,
              borderRadius: '50%', minWidth: 14, height: 14,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              padding: '0 3px',
            }}>
              {mentionCount > 99 ? '99+' : mentionCount}
            </span>
          )}
        </button>
        <button
          onClick={onToggleSearch}
          style={{
            ...styles.iconBtn,
            background: showSearch ? '#334155' : 'none',
            fontSize: '0.9rem',
            padding: '4px 8px',
          }}
          title="Search messages"
        >
          🔍
        </button>
        <button
          onClick={onTogglePins}
          style={{
            ...styles.iconBtn,
            background: showPins ? '#334155' : 'none',
            fontSize: '0.9rem',
            padding: '4px 8px',
          }}
          title="Pinned messages"
        >
          📌
        </button>
        <button
          onClick={onToggleParticipants}
          style={{
            ...styles.iconBtn,
            background: showParticipants ? '#334155' : 'none',
            fontSize: '0.9rem',
            padding: '4px 8px',
            position: 'relative',
          }}
          title={`Members${onlineUsers?.length ? ` (${onlineUsers.length} online)` : ''}`}
        >
          👥
          {onlineUsers?.length > 0 && (
            <span style={{
              position: 'absolute', top: 0, right: 0,
              background: '#34d399', color: '#0f172a',
              fontSize: '0.6rem', fontWeight: 700,
              borderRadius: '50%', minWidth: 14, height: 14,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              padding: '0 3px',
            }}>
              {onlineUsers.length}
            </span>
          )}
        </button>
        <button
          onClick={onOpenSettings}
          style={{
            ...styles.iconBtn,
            background: showSettings ? '#334155' : 'none',
            fontSize: '0.9rem',
            padding: '4px 8px',
          }}
          title="Room settings"
        >
          ⚙️
        </button>
        <button
          onClick={onToggleSound}
          style={{
            ...styles.iconBtn,
            fontSize: '0.9rem',
            padding: '4px 8px',
          }}
          title={soundEnabled ? 'Mute notifications' : 'Unmute notifications'}
        >
          {soundEnabled ? '🔔' : '🔕'}
        </button>
      </div>
    </div>
  );
}
