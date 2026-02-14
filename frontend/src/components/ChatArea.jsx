import React, { useState, useRef, useEffect, useCallback } from 'react';
import { styles } from '../styles';
import { API, formatDate, senderColor } from '../utils';
import ChatLogo from './ChatLogo';
import MentionsPanel from './MentionsPanel';
import MentionAutocomplete from './MentionAutocomplete';
import SearchPanel from './SearchPanel';
import RoomSettingsModal from './RoomSettingsModal';
import ParticipantPanel from './ParticipantPanel';
import PinnedPanel from './PinnedPanel';
import ThreadPanel from './ThreadPanel';
import TypingIndicator from './TypingIndicator';
import DateSeparator from './DateSeparator';
import FileCard from './FileCard';
import MessageGroup from './MessageGroup';

export default function ChatArea({ room, messages, files, sender, reactions, onSend, onEditMessage, onDeleteMessage, onDeleteFile, onUploadFile, onReact, onPin, onUnpin, adminKey, onTyping, typingUsers, loading, connected, rooms, onSelectRoom, onRoomUpdate, onRoomArchived, soundEnabled, onToggleSound, hasMore, onLoadOlder, onlineUsers }) {
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
  const [threadMessageId, setThreadMessageId] = useState(null);
  const [showMentions, setShowMentions] = useState(false);
  const [mentionCount, setMentionCount] = useState(0);
  const [participants, setParticipants] = useState([]);
  const [mentionQuery, setMentionQuery] = useState(null); // null = closed, string = active query
  const [mentionIndex, setMentionIndex] = useState(0);
  const mentionStartRef = useRef(null); // cursor position where @ was typed

  useEffect(() => {
    setReplyTo(null);
    setNewMsgCount(0);
    setLoadingOlder(false);
    setThreadMessageId(null);
  }, [room?.id]);

  // Poll unread mention count
  useEffect(() => {
    if (!sender) return;
    const fetchMentionCount = async () => {
      try {
        const res = await fetch(`${API}/mentions/unread?target=${encodeURIComponent(sender)}`);
        if (res.ok) {
          const data = await res.json();
          setMentionCount(data.total_unread || 0);
        }
      } catch (e) { /* ignore */ }
    };
    fetchMentionCount();
    const interval = setInterval(fetchMentionCount, 30000);
    return () => clearInterval(interval);
  }, [sender]);

  // Fetch participants for autocomplete
  useEffect(() => {
    if (!room?.id) { setParticipants([]); return; }
    const fetchParticipants = async () => {
      try {
        const res = await fetch(`${API}/rooms/${room.id}/participants`);
        if (res.ok) {
          const data = await res.json();
          setParticipants(data);
        }
      } catch (e) { /* ignore */ }
    };
    fetchParticipants();
    // Re-fetch every 60s while room is active
    const interval = setInterval(fetchParticipants, 60000);
    return () => clearInterval(interval);
  }, [room?.id]);

  // Close mention autocomplete when room changes
  useEffect(() => {
    setMentionQuery(null);
    setMentionIndex(0);
    mentionStartRef.current = null;
  }, [room?.id]);

  // Get filtered participants for the current mention query
  const filteredMentions = mentionQuery !== null
    ? participants.filter(p => p.sender.toLowerCase().includes(mentionQuery.toLowerCase()))
    : [];

  const handleMentionSelect = (name) => {
    if (!inputRef.current || mentionStartRef.current === null) return;
    const el = inputRef.current;
    const before = text.slice(0, mentionStartRef.current); // text before the @
    const after = text.slice(mentionStartRef.current + 1 + (mentionQuery || '').length); // text after the query
    const newText = before + '@' + name + ' ' + after;
    setText(newText);
    setMentionQuery(null);
    setMentionIndex(0);
    mentionStartRef.current = null;
    // Set cursor position after the inserted mention
    requestAnimationFrame(() => {
      const pos = before.length + 1 + name.length + 1;
      el.setSelectionRange(pos, pos);
      el.focus();
      autoResize(el);
    });
  };

  // Detect @ mentions while typing
  const detectMention = (value, cursorPos) => {
    // Look backward from cursor to find an unmatched @
    const textBeforeCursor = value.slice(0, cursorPos);
    const atIndex = textBeforeCursor.lastIndexOf('@');
    if (atIndex === -1) {
      setMentionQuery(null);
      mentionStartRef.current = null;
      return;
    }
    // @ must be at start or preceded by whitespace
    if (atIndex > 0 && !/\s/.test(textBeforeCursor[atIndex - 1])) {
      setMentionQuery(null);
      mentionStartRef.current = null;
      return;
    }
    const query = textBeforeCursor.slice(atIndex + 1);
    // No spaces in mention query (means they finished typing)
    if (/\s/.test(query)) {
      setMentionQuery(null);
      mentionStartRef.current = null;
      return;
    }
    // Max query length (prevent matching entire messages)
    if (query.length > 30) {
      setMentionQuery(null);
      mentionStartRef.current = null;
      return;
    }
    mentionStartRef.current = atIndex;
    setMentionQuery(query);
    setMentionIndex(0);
  };

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
    const value = e.target.value;
    setText(value);
    autoResize(e.target);
    if (value.trim()) {
      onTyping();
    }
    // Detect mention autocomplete
    detectMention(value, e.target.selectionStart);
  };

  const handleKeyDown = (e) => {
    // Handle mention autocomplete keyboard navigation
    if (mentionQuery !== null && filteredMentions.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setMentionIndex(prev => (prev + 1) % filteredMentions.length);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setMentionIndex(prev => (prev - 1 + filteredMentions.length) % filteredMentions.length);
        return;
      }
      if (e.key === 'Enter' || e.key === 'Tab') {
        e.preventDefault();
        handleMentionSelect(filteredMentions[mentionIndex].sender);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        setMentionQuery(null);
        mentionStartRef.current = null;
        return;
      }
    }
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
            onClick={() => setShowMentions(prev => !prev)}
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

      {showMentions && (
        <MentionsPanel
          sender={sender}
          onClose={() => setShowMentions(false)}
          rooms={rooms || []}
          onSelectRoom={(room) => { onSelectRoom?.(room); setShowMentions(false); }}
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

      {threadMessageId && room && (
        <ThreadPanel
          roomId={room.id}
          messageId={threadMessageId}
          sender={sender}
          onReply={handleReply}
          onClose={() => setThreadMessageId(null)}
        />
      )}

      {showSettings && (
        <RoomSettingsModal
          room={room}
          onClose={() => setShowSettings(false)}
          onUpdated={(updated) => { onRoomUpdate?.(updated); }}
          onRoomArchived={(updated) => { onRoomArchived?.(updated); }}
        />
      )}

      {!showSearch && !showMentions && (
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
              onOpenThread={(msgId) => setThreadMessageId(msgId)}
            />
          );
        })}
        <div ref={messagesEndRef} />
      </div>
      {showParticipants && room && (
        <ParticipantPanel roomId={room.id} onClose={() => setShowParticipants(false)} onlineUsers={onlineUsers || []} />
      )}
      </div>
      )}

      {!showSearch && !showMentions && <TypingIndicator typingUsers={typingUsers} />}

      {!showSearch && !showMentions && !autoScroll && (
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

      {!showSearch && !showMentions && replyTo && (
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

      {!showSearch && !showMentions && (
      <form onSubmit={handleSubmit} style={{ ...styles.inputArea, position: 'relative' }}>
        {mentionQuery !== null && filteredMentions.length > 0 && (
          <MentionAutocomplete
            query={mentionQuery}
            participants={filteredMentions}
            activeIndex={mentionIndex}
            onSelect={handleMentionSelect}
            onClose={() => { setMentionQuery(null); mentionStartRef.current = null; }}
          />
        )}
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
          onClick={(e) => detectMention(text, e.target.selectionStart)}
          onBlur={() => {
            // Delay to allow click on autocomplete item (mouseDown prevents blur)
            setTimeout(() => { setMentionQuery(null); mentionStartRef.current = null; }, 200);
          }}
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
