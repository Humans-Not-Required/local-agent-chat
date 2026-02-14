import React, { useState, useEffect, useRef, useCallback } from 'react';
import { styles, injectGlobalStyles } from './styles';
import { API, senderColor } from './utils';
import { RoomList, ChatArea, SenderModal, AdminKeyModal, ProfileModal } from './components';
import { useSSE, useChatAPI } from './hooks';

// Inject global CSS on load
injectGlobalStyles();

export default function App() {
  const [sender, setSender] = useState(() => localStorage.getItem('chat-sender') || '');
  const [senderType, setSenderType] = useState(() => localStorage.getItem('chat-sender-type') || 'agent');
  const [rooms, setRooms] = useState([]);
  const [activeRoom, setActiveRoom] = useState(null);
  const [messages, setMessages] = useState([]);
  const [files, setFiles] = useState([]);
  const [reactions, setReactions] = useState({});
  const [adminKeyInfo, setAdminKeyInfo] = useState(null);
  const [adminKeys, setAdminKeys] = useState(() => {
    try { return JSON.parse(localStorage.getItem('chat-admin-keys') || '{}'); } catch { return {}; }
  });
  const [showSidebar, setShowSidebar] = useState(window.innerWidth > 768);
  const [typingUsers, setTypingUsers] = useState([]);
  const [unreadCounts, setUnreadCounts] = useState({});
  const [soundEnabled, setSoundEnabled] = useState(() => localStorage.getItem('chat-sound') !== 'off');
  const [onlineUsers, setOnlineUsers] = useState([]);
  const [showProfileModal, setShowProfileModal] = useState(false);
  const [dmConversations, setDmConversations] = useState([]);
  const [isDmView, setIsDmView] = useState(false);
  const [profiles, setProfiles] = useState({});

  const senderRef = useRef(sender);
  const soundEnabledRef = useRef(soundEnabled);
  const lastSeqRef = useRef(null);
  const typingTimeoutsRef = useRef({});
  const lastTypingSentRef = useRef(0);
  const activeRoomRef = useRef(null);

  const saveAdminKey = useCallback((roomId, key) => {
    setAdminKeys(prev => {
      const next = { ...prev, [roomId]: key };
      try { localStorage.setItem('chat-admin-keys', JSON.stringify(next)); } catch { /* ignore */ }
      return next;
    });
  }, []);

  // --- Hooks ---

  const api = useChatAPI({
    senderRef,
    lastSeqRef,
    setMessages,
    setFiles,
    setReactions,
    setRooms,
    setActiveRoom,
    setUnreadCounts,
    setDmConversations,
    saveAdminKey,
    setAdminKeyInfo,
  });

  const sse = useSSE({
    senderRef,
    soundEnabledRef,
    lastSeqRef,
    activeRoomRef,
    typingTimeoutsRef,
    setMessages,
    setFiles,
    setReactions,
    setOnlineUsers,
    setTypingUsers,
    setRooms,
    setActiveRoom,
    setProfiles,
    markRoomRead: api.markRoomRead,
  });

  // --- Ref Sync ---

  useEffect(() => { senderRef.current = sender; }, [sender]);
  useEffect(() => {
    soundEnabledRef.current = soundEnabled;
    localStorage.setItem('chat-sound', soundEnabled ? 'on' : 'off');
  }, [soundEnabled]);

  // --- Tab title with unread count ---

  useEffect(() => {
    const total = Object.values(unreadCounts).reduce((sum, n) => sum + n, 0);
    document.title = total > 0 ? `(${total}) Local Agent Chat` : 'Local Agent Chat';
  }, [unreadCounts]);

  // --- Initial data load + polling ---

  // Fetch all profiles on mount and refresh periodically
  const fetchProfiles = useCallback(async () => {
    try {
      const res = await fetch(`${API}/profiles`);
      if (res.ok) {
        const data = await res.json();
        const map = {};
        data.forEach(p => { map[p.sender] = p; });
        setProfiles(map);
      }
    } catch (e) { /* ignore */ }
  }, []);

  useEffect(() => {
    api.fetchRooms().then(data => {
      if (data.length > 0 && !activeRoom) {
        const general = data.find(r => r.name === 'general') || data[0];
        setActiveRoom(general);
      }
    });
    api.fetchUnread();
    api.fetchDmConversations();
    fetchProfiles();
    const roomInterval = setInterval(api.fetchRooms, 30000);
    const unreadInterval = setInterval(api.fetchUnread, 30000);
    const dmInterval = setInterval(api.fetchDmConversations, 30000);
    const profileInterval = setInterval(fetchProfiles, 60000);
    return () => { clearInterval(roomInterval); clearInterval(unreadInterval); clearInterval(dmInterval); clearInterval(profileInterval); };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // --- Active room: load data + connect SSE ---

  useEffect(() => {
    if (!activeRoom) return;
    activeRoomRef.current = activeRoom;
    lastSeqRef.current = null;
    setTypingUsers([]);
    setOnlineUsers([]);
    setFiles([]);
    Object.values(typingTimeoutsRef.current).forEach(clearTimeout);
    typingTimeoutsRef.current = {};
    setReactions({});
    Promise.all([
      api.fetchMessages(activeRoom.id),
      api.fetchFiles(activeRoom.id),
      api.fetchReactions(activeRoom.id),
    ]).then(() => {
      if (lastSeqRef.current) {
        api.markRoomRead(activeRoom.id, lastSeqRef.current);
      }
      sse.connect(activeRoom.id);
    });
    return () => { sse.disconnect(); };
  }, [activeRoom?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // --- Window resize + visibility handlers ---

  useEffect(() => {
    const handle = () => {
      if (window.innerWidth > 768) setShowSidebar(true);
    };
    window.addEventListener('resize', handle);

    const handleVisibility = () => {
      if (!document.hidden && activeRoomRef.current && lastSeqRef.current) {
        api.markRoomRead(activeRoomRef.current.id, lastSeqRef.current);
      }
    };
    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      window.removeEventListener('resize', handle);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // --- Event Handlers (thin wrappers binding current state) ---

  const handleSetSender = (name, type) => {
    localStorage.setItem('chat-sender', name);
    localStorage.setItem('chat-sender-type', type || 'agent');
    setSender(name);
    setSenderType(type || 'agent');
  };

  const handleSelectDm = useCallback((conv) => {
    setIsDmView(true);
    setActiveRoom({
      id: conv.room_id,
      name: conv.other_participant,
      description: `DM with ${conv.other_participant}`,
      created_by: sender,
      message_count: conv.message_count,
      isDm: true,
      other_participant: conv.other_participant,
    });
    if (window.innerWidth <= 768) setShowSidebar(false);
  }, [sender]);

  const handleSelectRoom = (room) => {
    setIsDmView(false);
    setActiveRoom(room);
    if (window.innerWidth <= 768) setShowSidebar(false);
  };

  const onStartDm = useCallback(async (recipient, content) => {
    const result = await api.handleStartDm(sender, senderType, recipient, content);
    if (result) {
      setIsDmView(true);
      setActiveRoom({
        id: result.room_id,
        name: result.recipient,
        description: `DM with ${result.recipient}`,
        created_by: sender,
        isDm: true,
        other_participant: result.recipient,
      });
    }
  }, [sender, senderType, api]);

  // --- Render ---

  if (!sender) {
    return <SenderModal onSet={handleSetSender} />;
  }

  return (
    <div style={styles.container}>
      {adminKeyInfo && (
        <AdminKeyModal
          roomName={adminKeyInfo.roomName}
          adminKey={adminKeyInfo.adminKey}
          onDismiss={() => setAdminKeyInfo(null)}
        />
      )}
      {showProfileModal && (
        <ProfileModal
          sender={sender}
          onClose={() => setShowProfileModal(false)}
        />
      )}
      {/* Mobile header */}
      <div className="chat-mobile-header" data-mobile-header style={styles.mobileHeader}>
        <button onClick={() => setShowSidebar(!showSidebar)} style={styles.iconBtn}>
          {showSidebar ? '✕' : '☰'}
        </button>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, flex: 1, minWidth: 0 }}>
          <span style={{ fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {activeRoom ? (isDmView ? `💬 ${activeRoom.name}` : `#${activeRoom.name}`) : 'Local Agent Chat'}
          </span>
          {sse.connected && (
            <div style={{ width: 6, height: 6, borderRadius: '50%', background: '#34d399', flexShrink: 0 }} />
          )}
        </div>
        <span style={{ fontSize: '0.7rem', color: senderColor(sender), whiteSpace: 'nowrap', flexShrink: 0 }}>
          {senderType === 'human' ? '👤' : '🤖'} {sender}
        </span>
      </div>

      <div style={styles.main}>
        {showSidebar && (
          <>
            <div
              className="chat-sidebar-backdrop"
              onClick={() => { if (window.innerWidth <= 768) setShowSidebar(false); }}
            />
            <RoomList
              rooms={rooms}
              activeRoom={activeRoom}
              onSelect={handleSelectRoom}
              onCreateRoom={(name, desc) => api.handleCreateRoom(name, desc, sender)}
              unreadCounts={unreadCounts}
              sender={sender}
              senderType={senderType}
              senderProfile={profiles[sender]}
              onChangeSender={() => {
                localStorage.removeItem('chat-sender');
                localStorage.removeItem('chat-sender-type');
                setSender('');
                setSenderType('agent');
              }}
              onEditProfile={() => setShowProfileModal(true)}
              dmConversations={dmConversations}
              onSelectDm={handleSelectDm}
              onStartDm={onStartDm}
            />
          </>
        )}
        <ChatArea
          room={activeRoom}
          messages={messages}
          files={files}
          sender={sender}
          reactions={reactions}
          profiles={profiles}
          onSend={(content, replyToId) => api.handleSend(activeRoom, sender, senderType, content, replyToId, isDmView)}
          onEditMessage={(msgId, newContent) => api.handleEditMessage(activeRoom, sender, msgId, newContent)}
          onDeleteMessage={(msgId) => api.handleDeleteMessage(activeRoom, sender, msgId)}
          onDeleteFile={(fileId) => api.handleDeleteFile(activeRoom, sender, fileId)}
          onUploadFile={(name, ct, data) => api.handleUploadFile(activeRoom, sender, name, ct, data)}
          onReact={(msgId, emoji) => api.handleToggleReaction(activeRoom, sender, msgId, emoji)}
          onPin={(msgId) => api.handlePinMessage(activeRoom, adminKeys, msgId)}
          onUnpin={(msgId) => api.handleUnpinMessage(activeRoom, adminKeys, msgId)}
          adminKey={activeRoom ? adminKeys[activeRoom.id] : null}
          onTyping={() => api.handleTyping(activeRoom, sender, lastTypingSentRef)}
          typingUsers={typingUsers}
          loading={api.loading}
          connected={sse.connected}
          rooms={rooms}
          onSelectRoom={handleSelectRoom}
          onRoomUpdate={api.handleRoomUpdate}
          onRoomArchived={api.handleRoomArchived}
          soundEnabled={soundEnabled}
          onToggleSound={() => setSoundEnabled(prev => !prev)}
          hasMore={api.hasMore}
          onLoadOlder={() => api.loadOlderMessages(activeRoom, messages)}
          onlineUsers={onlineUsers}
        />
      </div>
      <footer style={{ textAlign: 'center', padding: '4px 16px', fontSize: '0.6rem', color: '#475569', flexShrink: 0, borderTop: '1px solid #1e293b' }}>
        Made for AI, by AI.{' '}
        <a href="https://github.com/Humans-Not-Required" target="_blank" rel="noopener noreferrer" style={{ color: '#6366f1', textDecoration: 'none' }}>Humans not required</a>.
      </footer>
    </div>
  );
}
