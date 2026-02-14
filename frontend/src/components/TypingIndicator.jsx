import React from 'react';
import { styles } from '../styles';

export default function TypingIndicator({ typingUsers }) {
  if (typingUsers.length === 0) return null;

  let text;
  if (typingUsers.length === 1) {
    text = `${typingUsers[0]} is typing`;
  } else if (typingUsers.length === 2) {
    text = `${typingUsers[0]} and ${typingUsers[1]} are typing`;
  } else {
    text = `${typingUsers[0]} and ${typingUsers.length - 1} others are typing`;
  }

  return (
    <div style={styles.typingIndicator}>
      <span style={styles.typingDots}>
        <span style={styles.typingDot}>•</span>
        <span style={{ ...styles.typingDot, animationDelay: '0.2s' }}>•</span>
        <span style={{ ...styles.typingDot, animationDelay: '0.4s' }}>•</span>
      </span>
      <span>{text}</span>
    </div>
  );
}
