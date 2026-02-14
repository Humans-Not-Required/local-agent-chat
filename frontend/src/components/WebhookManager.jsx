import React, { useState, useEffect, useCallback } from 'react';
import { styles } from '../styles';
import { API, timeAgo } from '../utils';

const EVENTS = [
  'message', 'message_edited', 'message_deleted',
  'file_uploaded', 'file_deleted',
  'reaction_added', 'reaction_removed',
  'message_pinned', 'message_unpinned',
  'presence_joined', 'presence_left',
  'room_updated',
];

function CopyButton({ text }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = () => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };
  return (
    <button
      onClick={handleCopy}
      style={{ ...btnSmall, color: copied ? '#34d399' : '#94a3b8' }}
      title="Copy to clipboard"
    >
      {copied ? '✓' : '📋'}
    </button>
  );
}

const btnSmall = {
  background: 'none',
  border: 'none',
  cursor: 'pointer',
  padding: '2px 6px',
  fontSize: '0.8rem',
  borderRadius: 4,
};

const sectionStyle = {
  marginTop: 16,
  padding: '12px 0',
  borderTop: '1px solid #334155',
};

const listItemStyle = {
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  padding: '8px 10px',
  background: '#0f172a',
  borderRadius: 6,
  marginBottom: 6,
  fontSize: '0.8rem',
  color: '#e2e8f0',
};

const badgeStyle = {
  display: 'inline-block',
  padding: '1px 6px',
  borderRadius: 4,
  fontSize: '0.65rem',
  fontWeight: 600,
};

export default function WebhookManager({ roomId, adminKey }) {
  const [outgoing, setOutgoing] = useState([]);
  const [incoming, setIncoming] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  // Outgoing form
  const [showOutForm, setShowOutForm] = useState(false);
  const [outUrl, setOutUrl] = useState('');
  const [outEvents, setOutEvents] = useState('*');
  const [outSecret, setOutSecret] = useState('');
  const [outSaving, setOutSaving] = useState(false);

  // Incoming form
  const [showInForm, setShowInForm] = useState(false);
  const [inName, setInName] = useState('');
  const [inSaving, setInSaving] = useState(false);

  const headers = useCallback(() => ({
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${adminKey}`,
  }), [adminKey]);

  const fetchWebhooks = useCallback(async () => {
    if (!adminKey) { setLoading(false); return; }
    setLoading(true);
    setError('');
    try {
      const [outRes, inRes] = await Promise.all([
        fetch(`${API}/rooms/${roomId}/webhooks`, { headers: { 'Authorization': `Bearer ${adminKey}` } }),
        fetch(`${API}/rooms/${roomId}/incoming-webhooks`, { headers: { 'Authorization': `Bearer ${adminKey}` } }),
      ]);
      if (outRes.ok) setOutgoing(await outRes.json());
      else if (outRes.status === 401) { setError('Invalid admin key'); setLoading(false); return; }
      if (inRes.ok) setIncoming(await inRes.json());
    } catch {
      setError('Failed to load webhooks');
    }
    setLoading(false);
  }, [roomId, adminKey]);

  useEffect(() => { fetchWebhooks(); }, [fetchWebhooks]);

  // --- Outgoing webhook CRUD ---

  const handleCreateOutgoing = async (e) => {
    e.preventDefault();
    if (!outUrl.trim()) return;
    setOutSaving(true);
    setError('');
    try {
      const body = { url: outUrl.trim(), events: outEvents.trim() || '*' };
      if (outSecret.trim()) body.secret = outSecret.trim();
      const res = await fetch(`${API}/rooms/${roomId}/webhooks`, {
        method: 'POST',
        headers: headers(),
        body: JSON.stringify(body),
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        setError(data.error || `Failed (${res.status})`);
      } else {
        setOutUrl('');
        setOutEvents('*');
        setOutSecret('');
        setShowOutForm(false);
        fetchWebhooks();
      }
    } catch {
      setError('Network error');
    }
    setOutSaving(false);
  };

  const handleToggleOutgoing = async (webhook) => {
    try {
      await fetch(`${API}/rooms/${roomId}/webhooks/${webhook.id}`, {
        method: 'PUT',
        headers: headers(),
        body: JSON.stringify({ active: !webhook.active }),
      });
      fetchWebhooks();
    } catch { /* ignore */ }
  };

  const handleDeleteOutgoing = async (webhookId) => {
    if (!window.confirm('Delete this webhook?')) return;
    try {
      await fetch(`${API}/rooms/${roomId}/webhooks/${webhookId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${adminKey}` },
      });
      fetchWebhooks();
    } catch { /* ignore */ }
  };

  // --- Incoming webhook CRUD ---

  const handleCreateIncoming = async (e) => {
    e.preventDefault();
    if (!inName.trim()) return;
    setInSaving(true);
    setError('');
    try {
      const res = await fetch(`${API}/rooms/${roomId}/incoming-webhooks`, {
        method: 'POST',
        headers: headers(),
        body: JSON.stringify({ name: inName.trim() }),
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        setError(data.error || `Failed (${res.status})`);
      } else {
        setInName('');
        setShowInForm(false);
        fetchWebhooks();
      }
    } catch {
      setError('Network error');
    }
    setInSaving(false);
  };

  const handleToggleIncoming = async (webhook) => {
    try {
      await fetch(`${API}/rooms/${roomId}/incoming-webhooks/${webhook.id}`, {
        method: 'PUT',
        headers: headers(),
        body: JSON.stringify({ active: !webhook.active }),
      });
      fetchWebhooks();
    } catch { /* ignore */ }
  };

  const handleDeleteIncoming = async (webhookId) => {
    if (!window.confirm('Delete this incoming webhook? Any integrations using its token will stop working.')) return;
    try {
      await fetch(`${API}/rooms/${roomId}/incoming-webhooks/${webhookId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${adminKey}` },
      });
      fetchWebhooks();
    } catch { /* ignore */ }
  };

  if (!adminKey) {
    return (
      <div style={{ color: '#64748b', fontSize: '0.8rem', textAlign: 'center', padding: '24px 0' }}>
        Enter an admin key in the General tab to manage webhooks.
      </div>
    );
  }

  if (loading) {
    return (
      <div style={{ color: '#64748b', fontSize: '0.8rem', textAlign: 'center', padding: '24px 0' }}>
        Loading webhooks...
      </div>
    );
  }

  return (
    <div>
      {error && <p style={{ color: '#ef4444', fontSize: '0.8rem', marginBottom: 8 }}>{error}</p>}

      {/* --- Outgoing Webhooks --- */}
      <div>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
          <h3 style={{ fontSize: '0.85rem', fontWeight: 600, color: '#e2e8f0', margin: 0 }}>
            🔔 Outgoing Webhooks
          </h3>
          <button
            onClick={() => setShowOutForm(!showOutForm)}
            style={{ ...btnSmall, color: '#3b82f6', fontWeight: 600 }}
          >
            {showOutForm ? '✕ Cancel' : '+ Add'}
          </button>
        </div>
        <p style={{ color: '#64748b', fontSize: '0.7rem', marginBottom: 8, marginTop: 0 }}>
          Send event notifications to external URLs when things happen in this room.
        </p>

        {showOutForm && (
          <form onSubmit={handleCreateOutgoing} style={{ background: '#0f172a', padding: 12, borderRadius: 8, marginBottom: 12, border: '1px solid #334155' }}>
            <label style={{ display: 'block', color: '#94a3b8', fontSize: '0.75rem', marginBottom: 4 }}>URL *</label>
            <input
              value={outUrl}
              onChange={e => setOutUrl(e.target.value)}
              placeholder="https://example.com/webhook"
              style={{ ...styles.input, marginBottom: 8, fontSize: '0.85rem' }}
              autoFocus
            />
            <label style={{ display: 'block', color: '#94a3b8', fontSize: '0.75rem', marginBottom: 4 }}>
              Events <span style={{ color: '#64748b' }}>(comma-separated, or * for all)</span>
            </label>
            <input
              value={outEvents}
              onChange={e => setOutEvents(e.target.value)}
              placeholder="* (all events)"
              style={{ ...styles.input, marginBottom: 8, fontSize: '0.85rem' }}
            />
            <label style={{ display: 'block', color: '#94a3b8', fontSize: '0.75rem', marginBottom: 4 }}>
              Secret <span style={{ color: '#64748b' }}>(optional, for HMAC signing)</span>
            </label>
            <input
              value={outSecret}
              onChange={e => setOutSecret(e.target.value)}
              placeholder="Optional HMAC secret"
              style={{ ...styles.input, marginBottom: 12, fontSize: '0.85rem' }}
            />
            <div style={{ display: 'flex', gap: 6 }}>
              <button type="submit" disabled={outSaving} style={{ ...styles.btnPrimary, fontSize: '0.8rem', padding: '6px 12px', opacity: outSaving ? 0.6 : 1 }}>
                {outSaving ? 'Creating...' : 'Create Webhook'}
              </button>
            </div>
          </form>
        )}

        {outgoing.length === 0 && !showOutForm && (
          <div style={{ color: '#475569', fontSize: '0.75rem', padding: '8px 0' }}>
            No outgoing webhooks configured.
          </div>
        )}

        {outgoing.map(wh => (
          <div key={wh.id} style={listItemStyle}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: '0.75rem' }}>
                {wh.url}
              </div>
              <div style={{ display: 'flex', gap: 4, marginTop: 4, flexWrap: 'wrap', alignItems: 'center' }}>
                <span style={{
                  ...badgeStyle,
                  background: wh.active ? '#065f4622' : '#7f1d1d22',
                  color: wh.active ? '#34d399' : '#f87171',
                }}>
                  {wh.active ? 'active' : 'paused'}
                </span>
                <span style={{ ...badgeStyle, background: '#1e293b', color: '#94a3b8' }}>
                  {wh.events === '*' ? 'all events' : wh.events}
                </span>
                {wh.secret && (
                  <span style={{ ...badgeStyle, background: '#1e293b', color: '#a78bfa' }}>
                    🔐 signed
                  </span>
                )}
              </div>
            </div>
            <button
              onClick={() => handleToggleOutgoing(wh)}
              style={{ ...btnSmall, color: wh.active ? '#f59e0b' : '#34d399' }}
              title={wh.active ? 'Pause' : 'Resume'}
            >
              {wh.active ? '⏸' : '▶'}
            </button>
            <button
              onClick={() => handleDeleteOutgoing(wh.id)}
              style={{ ...btnSmall, color: '#f87171' }}
              title="Delete"
            >
              🗑
            </button>
          </div>
        ))}
      </div>

      {/* --- Incoming Webhooks --- */}
      <div style={sectionStyle}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
          <h3 style={{ fontSize: '0.85rem', fontWeight: 600, color: '#e2e8f0', margin: 0 }}>
            📥 Incoming Webhooks
          </h3>
          <button
            onClick={() => setShowInForm(!showInForm)}
            style={{ ...btnSmall, color: '#3b82f6', fontWeight: 600 }}
          >
            {showInForm ? '✕ Cancel' : '+ Add'}
          </button>
        </div>
        <p style={{ color: '#64748b', fontSize: '0.7rem', marginBottom: 8, marginTop: 0 }}>
          External systems can post messages into this room using a simple token URL.
        </p>

        {showInForm && (
          <form onSubmit={handleCreateIncoming} style={{ background: '#0f172a', padding: 12, borderRadius: 8, marginBottom: 12, border: '1px solid #334155' }}>
            <label style={{ display: 'block', color: '#94a3b8', fontSize: '0.75rem', marginBottom: 4 }}>Name *</label>
            <input
              value={inName}
              onChange={e => setInName(e.target.value)}
              placeholder="e.g., CI Alerts, Monitoring"
              style={{ ...styles.input, marginBottom: 12, fontSize: '0.85rem' }}
              autoFocus
            />
            <button type="submit" disabled={inSaving} style={{ ...styles.btnPrimary, fontSize: '0.8rem', padding: '6px 12px', opacity: inSaving ? 0.6 : 1 }}>
              {inSaving ? 'Creating...' : 'Create Incoming Webhook'}
            </button>
          </form>
        )}

        {incoming.length === 0 && !showInForm && (
          <div style={{ color: '#475569', fontSize: '0.75rem', padding: '8px 0' }}>
            No incoming webhooks configured.
          </div>
        )}

        {incoming.map(wh => (
          <div key={wh.id} style={listItemStyle}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontWeight: 500, fontSize: '0.8rem' }}>
                {wh.name}
              </div>
              <div style={{ display: 'flex', gap: 4, marginTop: 4, alignItems: 'center' }}>
                <span style={{
                  ...badgeStyle,
                  background: wh.active !== false ? '#065f4622' : '#7f1d1d22',
                  color: wh.active !== false ? '#34d399' : '#f87171',
                }}>
                  {wh.active !== false ? 'active' : 'paused'}
                </span>
              </div>
              {wh.token && (
                <div style={{
                  marginTop: 6,
                  padding: '4px 8px',
                  background: '#1e293b',
                  borderRadius: 4,
                  fontSize: '0.65rem',
                  color: '#94a3b8',
                  fontFamily: 'monospace',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 4,
                }}>
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    POST /api/v1/hook/{wh.token}
                  </span>
                  <CopyButton text={`${window.location.origin}/api/v1/hook/${wh.token}`} />
                </div>
              )}
            </div>
            <button
              onClick={() => handleToggleIncoming(wh)}
              style={{ ...btnSmall, color: (wh.active !== false) ? '#f59e0b' : '#34d399' }}
              title={(wh.active !== false) ? 'Pause' : 'Resume'}
            >
              {(wh.active !== false) ? '⏸' : '▶'}
            </button>
            <button
              onClick={() => handleDeleteIncoming(wh.id)}
              style={{ ...btnSmall, color: '#f87171' }}
              title="Delete"
            >
              🗑
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
