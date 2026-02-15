import React, { useState, useEffect, useRef } from 'react';
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
  const [isMobile, setIsMobile] = useState(window.innerWidth <= 768);
  const [showOverflow, setShowOverflow] = useState(false);
  const overflowRef = useRef(null);

  useEffect(() => {
    const onResize = () => setIsMobile(window.innerWidth <= 768);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  // Close overflow on outside click
  useEffect(() => {
    if (!showOverflow) return;
    const onClick = (e) => {
      if (overflowRef.current && !overflowRef.current.contains(e.target)) {
        setShowOverflow(false);
      }
    };
    document.addEventListener('mousedown', onClick);
    return () => document.removeEventListener('mousedown', onClick);
  }, [showOverflow]);

  const iconBtnStyle = (active) => ({
    ...styles.iconBtn,
    background: active ? '#334155' : 'none',
    fontSize: '0.9rem',
    padding: '4px 8px',
  });

  const liveIndicator = (
    <div className="chat-live-indicator" style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      <div style={{
        width: 8, height: 8, borderRadius: '50%',
        background: connected ? '#34d399' : '#ef4444',
      }} />
      <span className="chat-live-text" style={{ fontSize: '0.75rem', color: '#64748b' }}>
        {connected ? 'Live' : 'Reconnecting...'}
      </span>
    </div>
  );

  const mentionsBtn = (
    <button
      onClick={() => { onToggleMentions(); setShowOverflow(false); }}
      style={{ ...iconBtnStyle(showMentions), position: 'relative' }}
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
  );

  const searchBtn = (
    <button
      onClick={() => { onToggleSearch(); setShowOverflow(false); }}
      style={iconBtnStyle(showSearch)}
      title="Search messages"
    >
      🔍
    </button>
  );

  const pinsBtn = (
    <button
      onClick={() => { onTogglePins(); setShowOverflow(false); }}
      style={iconBtnStyle(showPins)}
      title="Pinned messages"
    >
      📌
    </button>
  );

  const participantsBtn = (
    <button
      onClick={() => { onToggleParticipants(); setShowOverflow(false); }}
      style={{ ...iconBtnStyle(showParticipants), position: 'relative' }}
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
  );

  const settingsBtn = (
    <button
      onClick={() => { onOpenSettings(); setShowOverflow(false); }}
      style={iconBtnStyle(showSettings)}
      title="Room settings"
    >
      ⚙️
    </button>
  );

  const soundBtn = (
    <button
      onClick={() => { onToggleSound(); setShowOverflow(false); }}
      style={{ ...styles.iconBtn, fontSize: '0.9rem', padding: '4px 8px' }}
      title={soundEnabled ? 'Mute notifications' : 'Unmute notifications'}
    >
      {soundEnabled ? '🔔' : '🔕'}
    </button>
  );

  // Desktop: show all buttons inline
  if (!isMobile) {
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
          {liveIndicator}
          {mentionsBtn}
          {searchBtn}
          {pinsBtn}
          {participantsBtn}
          {settingsBtn}
          {soundBtn}
        </div>
      </div>
    );
  }

  // Mobile: show key buttons + overflow menu
  const hasUnread = mentionCount > 0;
  const hasActivePanels = showMentions || showPins || showSettings;

  return (
    <div className="chat-content-header" style={{
      ...styles.chatHeader,
      padding: '6px 12px',
    }}>
      <div className="chat-header-actions" style={{
        display: 'flex', alignItems: 'center', gap: 4,
        width: '100%', justifyContent: 'flex-end',
      }}>
        {liveIndicator}
        {searchBtn}
        {participantsBtn}
        <div ref={overflowRef} style={{ position: 'relative' }}>
          <button
            onClick={() => setShowOverflow(v => !v)}
            style={{
              ...iconBtnStyle(showOverflow || hasActivePanels),
              position: 'relative',
            }}
            title="More options"
          >
            ⋯
            {hasUnread && !showOverflow && (
              <span style={{
                position: 'absolute', top: -2, right: -2,
                background: '#7c3aed', width: 8, height: 8,
                borderRadius: '50%',
              }} />
            )}
          </button>
          {showOverflow && (
            <div style={{
              position: 'absolute', top: '100%', right: 0,
              marginTop: 4,
              background: '#1e293b',
              border: '1px solid #334155',
              borderRadius: 8,
              boxShadow: '0 8px 24px rgba(0,0,0,0.4)',
              zIndex: 100,
              minWidth: 160,
              padding: '4px 0',
            }}>
              <OverflowItem onClick={() => { onToggleMentions(); setShowOverflow(false); }} active={showMentions}>
                @ Mentions{mentionCount > 0 && <Badge count={mentionCount} />}
              </OverflowItem>
              <OverflowItem onClick={() => { onTogglePins(); setShowOverflow(false); }} active={showPins}>
                📌 Pins
              </OverflowItem>
              <OverflowItem onClick={() => { onOpenSettings(); setShowOverflow(false); }} active={showSettings}>
                ⚙️ Settings
              </OverflowItem>
              <OverflowItem onClick={() => { onToggleSound(); setShowOverflow(false); }}>
                {soundEnabled ? '🔔 Mute' : '🔕 Unmute'}
              </OverflowItem>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function OverflowItem({ onClick, active, children }) {
  return (
    <button
      onClick={onClick}
      style={{
        display: 'flex', alignItems: 'center', gap: 8,
        width: '100%', padding: '8px 12px',
        background: active ? '#334155' : 'transparent',
        border: 'none', color: '#e2e8f0',
        fontSize: '0.85rem', cursor: 'pointer',
        textAlign: 'left',
      }}
      onMouseEnter={(e) => { if (!active) e.target.style.background = '#2a3548'; }}
      onMouseLeave={(e) => { if (!active) e.target.style.background = 'transparent'; }}
    >
      {children}
    </button>
  );
}

function Badge({ count }) {
  return (
    <span style={{
      background: '#7c3aed', color: '#fff',
      fontSize: '0.65rem', fontWeight: 700,
      borderRadius: 8, padding: '1px 5px',
      marginLeft: 'auto',
    }}>
      {count > 99 ? '99+' : count}
    </span>
  );
}
