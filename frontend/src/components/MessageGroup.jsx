import React from 'react';
import { senderColor } from '../utils';
import MessageBubble from './MessageBubble';

export default function MessageGroup({ messages, isOwn, onEdit, onDelete, onReply, onReact, onPin, onUnpin, hasAdminKey, reactions, sender, allMessages, onOpenThread, profile }) {
  const msgSender = messages[0].sender;
  const color = senderColor(msgSender);
  const msgType = messages[0].sender_type || messages[0].metadata?.sender_type;
  const typeIcon = msgType === 'human' ? '👤' : msgType === 'agent' ? '🤖' : '';
  const displayName = profile?.display_name || msgSender;
  const avatarUrl = profile?.avatar_url;
  const initial = msgSender.charAt(0).toUpperCase();

  return (
    <div style={{ marginBottom: 16, display: 'flex', flexDirection: isOwn ? 'row-reverse' : 'row', alignItems: 'flex-start', gap: 8 }}>
      {/* Avatar */}
      <div style={{ flexShrink: 0, marginTop: 2 }}>
        {avatarUrl ? (
          <img
            src={avatarUrl}
            alt={msgSender}
            style={{
              width: 32, height: 32, borderRadius: '50%',
              objectFit: 'cover', background: '#1e293b',
            }}
            onError={(e) => { e.target.style.display = 'none'; e.target.nextSibling.style.display = 'flex'; }}
          />
        ) : null}
        <div
          style={{
            width: 32, height: 32, borderRadius: '50%',
            background: color, display: avatarUrl ? 'none' : 'flex',
            alignItems: 'center', justifyContent: 'center',
            fontSize: '0.85rem', fontWeight: 700, color: '#0f172a',
            userSelect: 'none',
          }}
        >
          {initial}
        </div>
      </div>
      {/* Messages */}
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: isOwn ? 'flex-end' : 'flex-start', minWidth: 0, flex: 1 }}>
        <div style={{ fontSize: '0.8rem', fontWeight: 600, color, marginBottom: 4, paddingLeft: isOwn ? 0 : 2, paddingRight: isOwn ? 2 : 0, display: 'flex', alignItems: 'center', gap: 4 }}>
          {typeIcon && <span>{typeIcon}</span>}
          <span>{displayName}</span>
          {displayName !== msgSender && <span style={{ fontSize: '0.7rem', color: '#64748b', fontWeight: 400 }}>@{msgSender}</span>}
        </div>
        {messages.map(msg => (
          <MessageBubble
            key={msg.id}
            msg={msg}
            isOwn={isOwn}
            onEdit={onEdit}
            onDelete={onDelete}
            onReply={onReply}
            onReact={onReact}
            onPin={onPin}
            onUnpin={onUnpin}
            hasAdminKey={hasAdminKey}
            reactions={(reactions || {})[msg.id] || []}
            sender={sender}
            allMessages={allMessages}
            onOpenThread={onOpenThread}
          />
        ))}
      </div>
    </div>
  );
}
