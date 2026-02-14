import React from 'react';

export default function ChatLogo({ size = 24, color = '#60a5fa', style: extraStyle }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      width={size}
      height={size}
      style={{ display: 'inline-block', verticalAlign: 'middle', flexShrink: 0, ...extraStyle }}
    >
      <circle cx="12" cy="12" r="10" stroke={color} strokeWidth="2" fill="none" />
      <path d="M8 10h0M12 10h0M16 10h0" stroke={color} strokeWidth="2" strokeLinecap="round" />
      <path d="M8 14c1.5 2 6.5 2 8 0" stroke={color} strokeWidth="2" strokeLinecap="round" fill="none" />
    </svg>
  );
}
