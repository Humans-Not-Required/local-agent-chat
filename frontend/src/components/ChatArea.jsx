import React, { useState, useRef, useEffect, useCallback } from 'react';
import { styles } from '../styles';
import { formatDate, senderColor } from '../utils';
import ChatLogo from './ChatLogo';
import SearchPanel from './SearchPanel';
import RoomSettingsModal from './RoomSettingsModal';
import ParticipantPanel from './ParticipantPanel';
import PinnedPanel from './PinnedPanel';
import TypingIndicator from './TypingIndicator';
import DateSeparator from './DateSeparator';
import FileCard from './FileCard';
import MessageGroup from './MessageGroup';

export default function ChatArea({ room, messages, files, sender, reactions, onSend, onEditMessage, onDeleteMessage, onDeleteFile, onUploadFile, onReact, onPin, onUnpin, adminKey, onTyping, typingUsers, loading, connected, rooms, onSelectRoom, onRoomUpdate, soundEnabled, onToggleSound, hasMore, onLoadOlder }) {
  const [text, setText] = useState('');
  const [replyTo, setReplyTo] = useState(null);
  const messagesEndRef = useRef(null);
  const containerRef = useRef(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const inputRef = useRef(null);
  const fileInputRef = useRef(null);
  const [uploading, setUploading] = useState(false);
  const [loadingOlder, setLoadingOlder] = useState(false);
  const [showParticipants, setShowParticipants] = useState(false);
  const [showSearch, setShowSearch] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showPins, setShowPins] = useState(false);
  const [newMsgCount, setNewMsgCount] = useState(0);
  const prevMsgCountRef = useRef(0);
  const [isDragging, setIsDragging] = useState(false);
  const dragCounterRef = useRef(0);

  useEffect(() => {
    setReplyTo(null);
    setNewMsgCount(0);
    setLoadingOlder(false);
  }, [room?.id]);

  const handleLoadOlder = async () => {
    if (loadingOlder || !onLoadOlder) return;
    setLoadingOlder(true);
    const el = containerRef.current;
    const prevScrollHeight = el ? el.scrollHeight : 0;
    await onLoadOlder();
    requestAnimationFrame(() => {
      if (el) {
        const newScrollHeight = el.scrollHeight;
        el.scrollTop = newScrollHeight - prevScrollHeight;
      }
      setLoadingOlder(false);
    });
  };

  useEffect(() => {
    const currentCount = messages.length;
    const delta = currentCount - prevMsgCountRef.current;
    if (delta > 0 && !autoScroll && prevMsgCountRef.current > 0) {
      setNewMsgCount(prev => prev + delta);
    }
    prevMsgCountRef.current = currentCount;
  }, [messages.length, autoScroll]);

  useEffect(() => {
    if (autoScroll) {
      setNewMsgCount(0);
    }
  }, [autoScroll]);

  useEffect(() => {
    if (autoScroll) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages, autoScroll]);

  const handleScroll = () => {
    const el = containerRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
    setAutoScroll(atBottom);
  };

  const handleSubmit = (e) => {
    e.preventDefault();
    if (!text.trim()) return;
    onSend(text.trim(), replyTo?.id || null);
    setText('');
    setReplyTo(null);
    if (inputRef.current) {
      inputRef.current.style.height = 'auto';
      inputRef.current.style.overflowY = 'hidden';
    }
  };

  useEffect(() => {
    const handler = (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setShowSearch(prev => !prev);
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  const handleReply = (msg) => {
    setReplyTo({ id: msg.id, sender: msg.sender, content: msg.content });
    inputRef.current?.focus();
  };

  const autoResize = (el) => {
    if (!el) return;
    el.style.height = 'auto';
    const maxHeight = 160;
    const h = el.scrollHeight + 2;
    el.style.height = Math.min(h, maxHeight) + 'px';
    el.style.overflowY = h > maxHeight ? 'auto' : 'hidden';
  };

  const handleTextChange = (e) => {
    setText(e.target.value);
    autoResize(e.target);
    if (e.target.value.trim()) {
      onTyping();
    }
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
    if (e.key === 'Escape' && replyTo) {
      setReplyTo(null);
    }
  };

  const handlePaste = async (e) => {
    const items = Array.from(e.clipboardData?.items || []);
    const imageItem = items.find(item => item.type.startsWith('image/'));
    if (!imageItem) return;

    e.preventDefault();
    const file = imageItem.getAsFile();
    if (!file) return;
    if (file.size > 5 * 1024 * 1024) {
      alert('Pasted image too large. Maximum size is 5 MB.');
      return;
    }

    setUploading(true);
    try {
      const reader = new FileReader();
      reader.onload = async () => {
        const base64 = reader.result.split(',')[1];
        const filename = `pasted-image-${Date.now()}.${file.type.split('/')[1] || 'png'}`;
        await onUploadFile(filename, file.type, base64);
        setUploading(false);
      };
      reader.onerror = () => setUploading(false);
      reader.readAsDataURL(file);
    } catch {
      setUploading(false);
    }
  };

  const handleFileSelect = async (e) => {
    const file = e.target.files?.[0];
    if (!file) return;
    e.target.value = '';

    if (file.size > 5 * 1024 * 1024) {
      alert('File too large. Maximum size is 5 MB.');
      return;
    }

    setUploading(true);
    try {
      const reader = new FileReader();
      reader.onload = async () => {
        const base64 = reader.result.split(',')[1];
        await onUploadFile(file.name, file.type || 'application/octet-stream', base64);
        setUploading(false);
      };
      reader.onerror = () => setUploading(false);
      reader.readAsDataURL(file);
    } catch {
      setUploading(false);
    }
  };

  const handleDragEnter = (e) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current++;
    if (e.dataTransfer?.types?.includes('Files')) {
      setIsDragging(true);
    }
  };

  const handleDragLeave = (e) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current--;
    if (dragCounterRef.current === 0) {
      setIsDragging(false);
    }
  };

  const handleDragOver = (e) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = async (e) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    dragCounterRef.current = 0;

    const droppedFiles = Array.from(e.dataTransfer?.files || []);
    if (droppedFiles.length === 0) return;

    const file = droppedFiles[0];
    if (file.size > 5 * 1024 * 1024) {
      alert('File too large. Maximum size is 5 MB.');
      return;
    }

    setUploading(true);
    try {
      const reader = new FileReader();
      reader.onload = async () => {
        const base64 = reader.result.split(',')[1];
        await onUploadFile(file.name, file.type || 'application/octet-stream', base64);
        setUploading(false);
      };
      reader.onerror = () => setUploading(false);
      reader.readAsDataURL(file);
    } catch {
      setUploading(false);
    }
  };

  if (!room) {
    return (
      <div style={styles.chatArea}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: '#64748b' }}>
          <div style={{ textAlign: 'center' }}>
            <div style={{ marginBottom: 12 }}>
              <ChatLogo size={56} />
            </div>
            <div style={{ fontSize: '1.1rem', fontWeight: 500 }}>Local Agent Chat</div>
            <div style={{ fontSize: '0.85rem', marginTop: 8 }}>Select a room to start chatting</div>
          </div>
        </div>
      </div>
    );
  }

  const timeline = [
    ...messages.map(m => ({ ...m, _type: 'message' })),
    ...(files || []).map(f => ({ ...f, _type: 'file' })),
  ].sort((a, b) => new Date(a.created_at) - new Date(b.created_at));

  const grouped = [];
  let currentGroup = null;
  let currentDate = null;

  for (const item of timeline) {
    const itemDate = formatDate(item.created_at);
    if (itemDate !== currentDate) {
      if (currentGroup) grouped.push(currentGroup);
      currentGroup = null;
      currentDate = itemDate;
      grouped.push({ type: 'date', date: itemDate });
    }
    if (item._type === 'file') {
      if (currentGroup) grouped.push(currentGroup);
      currentGroup = null;
      grouped.push({ type: 'file', file: item });
    } else {
      if (currentGroup && currentGroup.sender === item.sender) {
        currentGroup.messages.push(item);
      } else {
        if (currentGroup) grouped.push(currentGroup);
        currentGroup = { type: 'messages', sender: item.sender, messages: [item] };
      }
    }
  }
  if (currentGroup) grouped.push(currentGroup);

  return (
    <div
      style={styles.chatArea}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {isDragging && (
        <div style={{
          position: 'absolute', inset: 0, zIndex: 50,
          background: 'rgba(59, 130, 246, 0.15)',
          border: '3px dashed #3b82f6',
          borderRadius: 8,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          pointerEvents: 'none',
        }}>
          <div style={{
            background: '#1e293b', padding: '16px 32px', borderRadius: 12,
            display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8,
            boxShadow: '0 4px 24px rgba(0,0,0,0.4)',
          }}>
            <span style={{ fontSize: '2rem' }}>📎</span>
            <span style={{ color: '#f1f5f9', fontWeight: 600, fontSize: '1rem' }}>Drop file to upload</span>
            <span style={{ color: '#94a3b8', fontSize: '0.8rem' }}>Max 5 MB</span>
          </div>
        </div>
      )}
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
            onClick={() => setShowSearch(true)}
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
            onClick={() => setShowPins(prev => !prev)}
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
            onClick={() => setShowParticipants(prev => !prev)}
            style={{
              ...styles.iconBtn,
              background: showParticipants ? '#334155' : 'none',
              fontSize: '0.9rem',
              padding: '4px 8px',
            }}
            title="Members"
          >
            👥
          </button>
          <button
            onClick={() => setShowSettings(true)}
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

      {showSearch && (
        <SearchPanel
          onClose={() => setShowSearch(false)}
          rooms={rooms || []}
          onSelectRoom={(room) => { onSelectRoom?.(room); setShowSearch(false); }}
        />
      )}

      {showPins && room && (
        <PinnedPanel
          roomId={room.id}
          adminKey={adminKey}
          onUnpin={(msgId) => {
            onUnpin?.(msgId);
          }}
          onClose={() => setShowPins(false)}
        />
      )}

      {showSettings && (
        <RoomSettingsModal
          room={room}
          onClose={() => setShowSettings(false)}
          onUpdated={(updated) => { onRoomUpdate?.(updated); }}
        />
      )}

      {!showSearch && (
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
      <div ref={containerRef} onScroll={handleScroll} style={styles.messageContainer}>
        {loading && (
          <div style={{ textAlign: 'center', padding: 20, color: '#64748b' }}>Loading messages...</div>
        )}
        {!loading && messages.length === 0 && (
          <div style={{ textAlign: 'center', padding: 40, color: '#64748b' }}>
            <div style={{ fontSize: '1.5rem', marginBottom: 8 }}>🎉</div>
            <div>No messages yet. Be the first to say something!</div>
          </div>
        )}
        {!loading && hasMore && messages.length > 0 && (
          <div style={{ textAlign: 'center', padding: '12px 0 4px' }}>
            <button
              onClick={handleLoadOlder}
              disabled={loadingOlder}
              style={{
                background: 'rgba(255,255,255,0.06)',
                border: '1px solid rgba(255,255,255,0.1)',
                borderRadius: 6,
                color: '#94a3b8',
                padding: '6px 16px',
                fontSize: '0.8rem',
                cursor: loadingOlder ? 'default' : 'pointer',
                opacity: loadingOlder ? 0.6 : 1,
                transition: 'background 0.15s',
              }}
              onMouseEnter={e => { if (!loadingOlder) e.target.style.background = 'rgba(255,255,255,0.1)'; }}
              onMouseLeave={e => { e.target.style.background = 'rgba(255,255,255,0.06)'; }}
            >
              {loadingOlder ? '⏳ Loading...' : '↑ Load older messages'}
            </button>
          </div>
        )}
        {grouped.map((item, i) => {
          if (item.type === 'date') {
            return <DateSeparator key={`date-${i}`} date={item.date} />;
          }
          if (item.type === 'file') {
            return (
              <FileCard
                key={`file-${item.file.id}`}
                file={item.file}
                isOwn={item.file.sender === sender}
                sender={sender}
                onDelete={onDeleteFile}
              />
            );
          }
          return (
            <MessageGroup
              key={`group-${i}`}
              messages={item.messages}
              isOwn={item.sender === sender}
              onEdit={onEditMessage}
              onDelete={onDeleteMessage}
              onReply={handleReply}
              onReact={onReact}
              onPin={onPin}
              onUnpin={onUnpin}
              hasAdminKey={!!adminKey}
              reactions={reactions}
              sender={sender}
              allMessages={messages}
            />
          );
        })}
        <div ref={messagesEndRef} />
      </div>
      {showParticipants && room && (
        <ParticipantPanel roomId={room.id} onClose={() => setShowParticipants(false)} />
      )}
      </div>
      )}

      {!showSearch && <TypingIndicator typingUsers={typingUsers} />}

      {!showSearch && !autoScroll && (
        <button
          onClick={() => {
            messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
            setAutoScroll(true);
            setNewMsgCount(0);
          }}
          style={styles.scrollBtn}
        >
          {newMsgCount > 0
            ? `↓ ${newMsgCount} new message${newMsgCount === 1 ? '' : 's'}`
            : '↓ Jump to latest'}
        </button>
      )}

      {!showSearch && replyTo && (
        <div style={styles.replyBar}>
          <div style={{ width: 3, background: senderColor(replyTo.sender), borderRadius: 2, flexShrink: 0 }} />
          <div style={{ flex: 1, overflow: 'hidden' }}>
            <span style={{ fontSize: '0.75rem', fontWeight: 600, color: senderColor(replyTo.sender) }}>
              Replying to {replyTo.sender}
            </span>
            <div style={{ fontSize: '0.75rem', color: '#94a3b8', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {replyTo.content.length > 100 ? replyTo.content.slice(0, 100) + '…' : replyTo.content}
            </div>
          </div>
          <button onClick={() => setReplyTo(null)} style={styles.replyCloseBtn}>✕</button>
        </div>
      )}

      {!showSearch && (
      <form onSubmit={handleSubmit} style={styles.inputArea}>
        <input
          ref={fileInputRef}
          type="file"
          style={{ display: 'none' }}
          onChange={handleFileSelect}
        />
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          disabled={uploading}
          style={{
            ...styles.fileAttachBtn,
            opacity: uploading ? 0.5 : 1,
          }}
          title={uploading ? 'Uploading...' : 'Attach file (max 5 MB)'}
        >
          {uploading ? '⏳' : '📎'}
        </button>
        <textarea
          ref={inputRef}
          value={text}
          onChange={handleTextChange}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          placeholder={`Message #${room.name}...`}
          rows={1}
          style={styles.messageInput}
        />
        <button type="submit" disabled={!text.trim()} style={{
          ...styles.sendBtn,
          opacity: text.trim() ? 1 : 0.5,
        }}>
          Send
        </button>
      </form>
      )}
    </div>
  );
}
