import React from 'react';
import { senderColor } from '../utils';
import MessageBubble from './MessageBubble';

export default function MessageGroup({ messages, isOwn, onEdit, onDelete, onReply, onReact, reactions, sender, allMessages }) {
  const msgSender = messages[0].sender;
  const color = senderColor(msgSender);
  const msgType = messages[0].sender_type || messages[0].metadata?.sender_type;
  const typeIcon = msgType === 'human' ? '👤' : msgType === 'agent' ? '🤖' : '';

  return (
    <div style={{ marginBottom: 16, display: 'flex', flexDirection: 'column', alignItems: isOwn ? 'flex-end' : 'flex-start' }}>
      <div style={{ fontSize: '0.8rem', fontWeight: 600, color, marginBottom: 4, paddingLeft: isOwn ? 0 : 4, paddingRight: isOwn ? 4 : 0 }}>
        {typeIcon && <span style={{ marginRight: 4 }}>{typeIcon}</span>}{msgSender}
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
          reactions={(reactions || {})[msg.id] || []}
          sender={sender}
          allMessages={allMessages}
        />
      ))}
    </div>
  );
}
