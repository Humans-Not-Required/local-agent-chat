import React, { useState, useRef, useEffect, useCallback } from 'react';
import { styles } from '../styles';
import { API, groupTimeline } from '../utils';
import ChatLogo from './ChatLogo';
import ChatHeader from './ChatHeader';
import MessageInput from './MessageInput';
import MentionsPanel from './MentionsPanel';
import SearchPanel from './SearchPanel';
import RoomSettingsModal from './RoomSettingsModal';
import ParticipantPanel from './ParticipantPanel';
import PinnedPanel from './PinnedPanel';
import ThreadPanel from './ThreadPanel';
import TypingIndicator from './TypingIndicator';
import DateSeparator from './DateSeparator';
import FileCard from './FileCard';
import MessageGroup from './MessageGroup';
import useFileUpload from '../hooks/useFileUpload';

export default function ChatArea({ room, messages, files, sender, reactions, onSend, onEditMessage, onDeleteMessage, onDeleteFile, onUploadFile, onReact, onPin, onUnpin, adminKey, onTyping, typingUsers, loading, connected, rooms, onSelectRoom, onRoomUpdate, onRoomArchived, soundEnabled, onToggleSound, hasMore, onLoadOlder, onlineUsers }) {
  const [replyTo, setReplyTo] = useState(null);
  const messagesEndRef = useRef(null);
  const containerRef = useRef(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [loadingOlder, setLoadingOlder] = useState(false);
  const [showParticipants, setShowParticipants] = useState(false);
  const [showSearch, setShowSearch] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showPins, setShowPins] = useState(false);
  const [newMsgCount, setNewMsgCount] = useState(0);
  const prevMsgCountRef = useRef(0);
  const [threadMessageId, setThreadMessageId] = useState(null);
  const [showMentions, setShowMentions] = useState(false);
  const [mentionCount, setMentionCount] = useState(0);
  const [participants, setParticipants] = useState([]);

  const {
    uploading, isDragging, fileInputRef,
    handlePaste, handleFileSelect,
    handleDragEnter, handleDragLeave, handleDragOver, handleDrop,
  } = useFileUpload(onUploadFile);

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
        if (res.ok) setParticipants(await res.json());
      } catch (e) { /* ignore */ }
    };
    fetchParticipants();
    const interval = setInterval(fetchParticipants, 60000);
    return () => clearInterval(interval);
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

  // Track new messages for scroll-to-bottom badge
  useEffect(() => {
    const currentCount = messages.length;
    const delta = currentCount - prevMsgCountRef.current;
    if (delta > 0 && !autoScroll && prevMsgCountRef.current > 0) {
      setNewMsgCount(prev => prev + delta);
    }
    prevMsgCountRef.current = currentCount;
  }, [messages.length, autoScroll]);

  useEffect(() => {
    if (autoScroll) setNewMsgCount(0);
  }, [autoScroll]);

  useEffect(() => {
    if (autoScroll) messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, autoScroll]);

  const handleScroll = () => {
    const el = containerRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
    setAutoScroll(atBottom);
  };

  // Ctrl+K / Cmd+K search shortcut
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
  };

  // Empty state (no room selected)
  if (!room) {
    return (
      <div style={styles.chatArea}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: '#64748b' }}>
          <div style={{ textAlign: 'center' }}>
            <div style={{ marginBottom: 12 }}><ChatLogo size={56} /></div>
            <div style={{ fontSize: '1.1rem', fontWeight: 500 }}>Local Agent Chat</div>
            <div style={{ fontSize: '0.85rem', marginTop: 8 }}>Select a room to start chatting</div>
          </div>
        </div>
      </div>
    );
  }

  const grouped = groupTimeline(messages, files);

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

      <ChatHeader
        room={room}
        connected={connected}
        showMentions={showMentions}
        onToggleMentions={() => setShowMentions(prev => !prev)}
        mentionCount={mentionCount}
        showSearch={showSearch}
        onToggleSearch={() => setShowSearch(true)}
        showPins={showPins}
        onTogglePins={() => setShowPins(prev => !prev)}
        showParticipants={showParticipants}
        onToggleParticipants={() => setShowParticipants(prev => !prev)}
        onlineUsers={onlineUsers}
        showSettings={showSettings}
        onOpenSettings={() => setShowSettings(true)}
        soundEnabled={soundEnabled}
        onToggleSound={onToggleSound}
      />

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
          onUnpin={(msgId) => onUnpin?.(msgId)}
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
          onUpdated={(updated) => onRoomUpdate?.(updated)}
          onRoomArchived={(updated) => onRoomArchived?.(updated)}
          savedAdminKey={adminKey || ''}
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

      {!showSearch && !showMentions && (
        <MessageInput
          room={room}
          onSend={onSend}
          onTyping={onTyping}
          replyTo={replyTo}
          onCancelReply={() => setReplyTo(null)}
          uploading={uploading}
          fileInputRef={fileInputRef}
          onFileSelect={handleFileSelect}
          onPaste={handlePaste}
          participants={participants}
        />
      )}
    </div>
  );
}
