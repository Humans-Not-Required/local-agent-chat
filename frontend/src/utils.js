import React from 'react';

export function timeAgo(dateStr) {
  if (!dateStr) return '';
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

export function formatTime(dateStr) {
  if (!dateStr) return '';
  const d = new Date(dateStr);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

export function formatFullTimestamp(dateStr) {
  if (!dateStr) return '';
  const d = new Date(dateStr);
  return d.toLocaleString([], { weekday: 'short', year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

export function formatDate(dateStr) {
  if (!dateStr) return '';
  const d = new Date(dateStr);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);
  if (d.toDateString() === today.toDateString()) return 'Today';
  if (d.toDateString() === yesterday.toDateString()) return 'Yesterday';
  return d.toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' });
}

export function formatFileSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / 1048576).toFixed(1) + ' MB';
}

// Generate a consistent color for a sender name
export function senderColor(name) {
  if (!name) return '#94a3b8';
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  const colors = ['#f87171','#fb923c','#fbbf24','#a3e635','#34d399','#22d3ee','#60a5fa','#a78bfa','#f472b6','#e879f9'];
  return colors[Math.abs(hash) % colors.length];
}

// Convert URLs to clickable links and highlight @mentions
// Also handles inline markdown: bold, italic, strikethrough, inline code
export function linkify(text) {
  if (!text) return text;
  const tokenRegex = /(`[^`\n]+`|\*\*[^*\n]+\*\*|~~[^~\n]+~~|\*[^*\n]+\*|https?:\/\/[^\s<>"')\]]+|www\.[^\s<>"')\]]+|@[\w.-]+)/g;
  const parts = [];
  let lastIndex = 0;
  let match;
  let keyIdx = 0;

  while ((match = tokenRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index));
    }

    const token = match[0];

    if (token.startsWith('`') && token.endsWith('`')) {
      parts.push(
        React.createElement('code', {
          key: `code-${keyIdx++}`,
          style: { background: 'rgba(255,255,255,0.1)', padding: '1px 5px', borderRadius: 3, fontSize: '0.9em', fontFamily: "'Fira Code', 'Cascadia Code', 'Consolas', monospace" },
        }, token.slice(1, -1))
      );
    } else if (token.startsWith('**') && token.endsWith('**')) {
      parts.push(
        React.createElement('strong', {
          key: `bold-${keyIdx++}`,
        }, token.slice(2, -2))
      );
    } else if (token.startsWith('~~') && token.endsWith('~~')) {
      parts.push(
        React.createElement('del', {
          key: `strike-${keyIdx++}`,
          style: { opacity: 0.7 },
        }, token.slice(2, -2))
      );
    } else if (token.startsWith('*') && token.endsWith('*') && !token.startsWith('**')) {
      parts.push(
        React.createElement('em', {
          key: `italic-${keyIdx++}`,
        }, token.slice(1, -1))
      );
    } else if (token.startsWith('@')) {
      parts.push(
        React.createElement('span', {
          key: `mention-${keyIdx++}`,
          style: { color: '#a78bfa', fontWeight: 600, background: 'rgba(167,139,250,0.1)', borderRadius: 3, padding: '0 2px' },
        }, token)
      );
    } else {
      let url = token;
      const trailing = url.match(/[.,;:!?]+$/);
      let suffix = '';
      if (trailing) {
        suffix = trailing[0];
        url = url.slice(0, -suffix.length);
      }

      const href = url.startsWith('www.') ? 'https://' + url : url;
      parts.push(
        React.createElement('a', {
          key: `link-${keyIdx++}`,
          href: href,
          target: '_blank',
          rel: 'noopener noreferrer',
          style: { color: '#60a5fa', textDecoration: 'underline', wordBreak: 'break-all' },
          onClick: (e) => e.stopPropagation(),
        }, url)
      );
      if (suffix) parts.push(suffix);
    }

    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  return parts.length > 0 ? parts : text;
}

// Render block-level markdown: lists, blockquotes, horizontal rules
export function renderBlocks(text, keyPrefix) {
  if (!text) return [text];
  const lines = text.split('\n');
  const blocks = [];
  let i = 0;
  let keyIdx = 0;
  let regularLines = [];

  const kp = keyPrefix || 'blk';
  function flushRegular() {
    if (regularLines.length > 0) {
      const joined = regularLines.join('\n');
      blocks.push(React.createElement('span', { key: `${kp}-txt-${keyIdx++}` }, linkify(joined)));
      regularLines = [];
    }
  }

  while (i < lines.length) {
    const line = lines[i];

    // Horizontal rule
    if (/^([-*_])\1{2,}\s*$/.test(line.trim())) {
      flushRegular();
      blocks.push(React.createElement('hr', {
        key: `${kp}-hr-${keyIdx++}`,
        style: { border: 'none', borderTop: '1px solid rgba(255,255,255,0.2)', margin: '8px 0' },
      }));
      i++;
      continue;
    }

    // Blockquote
    if (/^>\s?/.test(line)) {
      flushRegular();
      const quoteLines = [];
      while (i < lines.length && /^>\s?/.test(lines[i])) {
        quoteLines.push(lines[i].replace(/^>\s?/, ''));
        i++;
      }
      blocks.push(React.createElement('blockquote', {
        key: `${kp}-bq-${keyIdx++}`,
        style: {
          borderLeft: '3px solid rgba(255,255,255,0.3)',
          paddingLeft: 12,
          margin: '4px 0',
          color: 'rgba(255,255,255,0.7)',
          fontStyle: 'italic',
          whiteSpace: 'pre-wrap',
        },
      }, linkify(quoteLines.join('\n'))));
      continue;
    }

    // Unordered list
    if (/^[-*]\s+/.test(line)) {
      flushRegular();
      const items = [];
      while (i < lines.length && /^[-*]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^[-*]\s+/, ''));
        i++;
      }
      blocks.push(React.createElement('ul', {
        key: `${kp}-ul-${keyIdx++}`,
        style: { margin: '4px 0', paddingLeft: 20 },
      }, items.map((item, j) =>
        React.createElement('li', { key: `${kp}-li-${keyIdx++}-${j}`, style: { marginBottom: 2 } }, linkify(item))
      )));
      continue;
    }

    // Ordered list
    if (/^\d+\.\s+/.test(line)) {
      flushRegular();
      const items = [];
      while (i < lines.length && /^\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+\.\s+/, ''));
        i++;
      }
      blocks.push(React.createElement('ol', {
        key: `${kp}-ol-${keyIdx++}`,
        style: { margin: '4px 0', paddingLeft: 20 },
      }, items.map((item, j) =>
        React.createElement('li', { key: `${kp}-oli-${keyIdx++}-${j}`, style: { marginBottom: 2 } }, linkify(item))
      )));
      continue;
    }

    regularLines.push(line);
    i++;
  }

  flushRegular();
  return blocks;
}

// Render message content with fenced code blocks + inline markdown
export function renderContent(text) {
  if (!text) return text;
  const codeBlockRegex = /```(\w*)\n?([\s\S]*?)```/g;
  const parts = [];
  let lastIndex = 0;
  let match;
  let keyIdx = 0;

  while ((match = codeBlockRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      const before = text.slice(lastIndex, match.index);
      parts.push(...renderBlocks(before, `pre-${keyIdx++}`));
    }

    const lang = match[1];
    const code = match[2];
    parts.push(
      React.createElement('div', {
        key: `codeblock-${keyIdx++}`,
        style: {
          background: 'rgba(0,0,0,0.35)',
          borderRadius: 6,
          padding: '10px 12px',
          margin: '6px 0',
          overflowX: 'auto',
          fontFamily: "'Fira Code', 'Cascadia Code', 'Consolas', monospace",
          fontSize: '0.85em',
          lineHeight: 1.5,
          whiteSpace: 'pre',
          wordBreak: 'normal',
          position: 'relative',
        },
      },
        lang ? React.createElement('div', {
          key: `lang-${keyIdx++}`,
          style: { fontSize: '0.7em', color: '#64748b', marginBottom: 4, textTransform: 'uppercase', letterSpacing: '0.05em' },
        }, lang) : null,
        React.createElement('code', null, code)
      )
    );

    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    const after = text.slice(lastIndex);
    parts.push(...renderBlocks(after, `post-${keyIdx++}`));
  }

  if (parts.length === 0) return renderBlocks(text);
  return parts;
}

// Notification sound using Web Audio API
let audioCtx = null;
export function playNotificationSound() {
  try {
    if (!audioCtx) audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    const now = audioCtx.currentTime;
    [0, 0.12].forEach((offset, i) => {
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.type = 'sine';
      osc.frequency.value = i === 0 ? 660 : 880;
      gain.gain.setValueAtTime(0.08, now + offset);
      gain.gain.exponentialRampToValueAtTime(0.001, now + offset + 0.15);
      osc.connect(gain);
      gain.connect(audioCtx.destination);
      osc.start(now + offset);
      osc.stop(now + offset + 0.15);
    });
  } catch { /* Audio not available */ }
}

export const API = '/api/v1';

/**
 * Groups a timeline of messages and files into renderable groups:
 * date separators, file cards, and consecutive message groups by sender.
 */
export function groupTimeline(messages, files) {
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

  return grouped;
}
