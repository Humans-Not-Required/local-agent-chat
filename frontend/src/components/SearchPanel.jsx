import React, { useState, useRef, useEffect, useCallback } from 'react';
import { styles } from '../styles';
import { API, timeAgo, formatFullTimestamp, senderColor } from '../utils';

export default function SearchPanel({ onClose, rooms, onSelectRoom }) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState([]);
  const [searching, setSearching] = useState(false);
  const [searched, setSearched] = useState(false);
  const inputRef = useRef(null);
  const debounceRef = useRef(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const doSearch = useCallback(async (q) => {
    if (!q.trim()) {
      setResults([]);
      setSearched(false);
      return;
    }
    setSearching(true);
    try {
      const res = await fetch(`${API}/search?q=${encodeURIComponent(q.trim())}&limit=50`);
      if (res.ok) {
        const data = await res.json();
        setResults(data);
      }
    } catch (e) { /* ignore */ }
    setSearching(false);
    setSearched(true);
  }, []);

  const handleChange = (e) => {
    const val = e.target.value;
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(val), 300);
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Escape') onClose();
    if (e.key === 'Enter') {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      doSearch(query);
    }
  };

  const handleResultClick = (result) => {
    const room = rooms.find(r => r.id === result.room_id);
    if (room) {
      onSelectRoom(room);
      onClose();
    }
  };

  const highlightMatch = (content, q) => {
    if (!q.trim() || !content) return content;
    const idx = content.toLowerCase().indexOf(q.toLowerCase());
    if (idx === -1) return content;
    const before = content.slice(0, idx);
    const match = content.slice(idx, idx + q.length);
    const after = content.slice(idx + q.length);
    return React.createElement(React.Fragment, null,
      before,
      React.createElement('mark', {
        style: { background: '#3b82f6', color: '#fff', borderRadius: 2, padding: '0 1px' }
      }, match),
      after
    );
  };

  return (
    <div style={styles.searchPanel}>
      <div style={styles.searchHeader}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}>
          <span style={{ color: '#64748b', fontSize: '1rem' }}>🔍</span>
          <input
            ref={inputRef}
            value={query}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            placeholder="Search messages across all rooms..."
            style={styles.searchInput}
          />
          {query && (
            <button
              onClick={() => { setQuery(''); setResults([]); setSearched(false); inputRef.current?.focus(); }}
              style={{ background: 'none', border: 'none', color: '#64748b', cursor: 'pointer', fontSize: '0.9rem', padding: '2px 6px' }}
            >✕</button>
          )}
        </div>
        <button onClick={onClose} style={styles.searchCloseBtn}>Close</button>
      </div>
      <div style={styles.searchResults}>
        {searching && (
          <div style={{ textAlign: 'center', padding: 20, color: '#64748b' }}>Searching...</div>
        )}
        {!searching && searched && results.length === 0 && (
          <div style={{ textAlign: 'center', padding: 40, color: '#64748b' }}>
            <div style={{ fontSize: '1.2rem', marginBottom: 8 }}>No results found</div>
            <div style={{ fontSize: '0.85rem' }}>Try different keywords</div>
          </div>
        )}
        {!searching && !searched && (
          <div style={{ textAlign: 'center', padding: 40, color: '#64748b' }}>
            <div style={{ fontSize: '1.2rem', marginBottom: 8 }}>🔍</div>
            <div style={{ fontSize: '0.85rem' }}>Search messages across all rooms</div>
            <div style={{ fontSize: '0.75rem', marginTop: 4 }}>Press Enter or just start typing</div>
          </div>
        )}
        {results.map(r => (
          <div
            key={r.id}
            onClick={() => handleResultClick(r)}
            style={styles.searchResultItem}
            onMouseEnter={e => e.currentTarget.style.background = '#1e293b'}
            onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
          >
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ fontSize: '0.75rem', color: '#3b82f6', fontWeight: 600 }}>
                  #{r.room_name || 'unknown'}
                </span>
                <span style={{ fontSize: '0.8rem', fontWeight: 600, color: senderColor(r.sender) }}>
                  {r.sender_type === 'human' ? '👤' : '🤖'} {r.sender}
                </span>
              </div>
              <span style={{ fontSize: '0.7rem', color: '#475569' }} title={formatFullTimestamp(r.created_at)}>{timeAgo(r.created_at)}</span>
            </div>
            <div style={{ fontSize: '0.85rem', color: '#cbd5e1', lineHeight: 1.4, overflow: 'hidden', textOverflow: 'ellipsis', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>
              {highlightMatch(r.content, query)}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
