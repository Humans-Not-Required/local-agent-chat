export const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    height: '100dvh',
    maxHeight: '100dvh',
    background: '#0f172a',
    overflow: 'hidden',
  },
  mobileHeader: {
    display: 'none',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '8px 12px',
    background: '#1e293b',
    borderBottom: '1px solid #334155',
  },
  main: {
    display: 'flex',
    flex: 1,
    overflow: 'hidden',
  },
  sidebar: {
    width: 260,
    minWidth: 260,
    background: '#0f172a',
    borderRight: '1px solid #1e293b',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  sidebarHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '16px 12px 12px',
    borderBottom: '1px solid #1e293b',
  },
  sidebarFooter: {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    padding: '10px 12px',
    borderTop: '1px solid #1e293b',
  },
  roomList: {
    flex: 1,
    overflowY: 'auto',
  },
  roomItem: {
    padding: '10px 12px',
    cursor: 'pointer',
    transition: 'background 0.15s',
  },
  createForm: {
    padding: 12,
    borderBottom: '1px solid #1e293b',
    display: 'flex',
    flexDirection: 'column',
    gap: 8,
  },
  chatArea: {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    position: 'relative',
    overflow: 'hidden',
  },
  chatHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '12px 16px',
    borderBottom: '1px solid #1e293b',
    background: '#0f172a',
  },
  messageContainer: {
    flex: 1,
    overflowY: 'auto',
    padding: '16px',
  },
  messageBubble: {
    padding: '8px 12px',
    marginBottom: 4,
    lineHeight: 1.5,
    fontSize: '0.9rem',
  },
  dateSeparator: {
    display: 'flex',
    alignItems: 'center',
    margin: '16px 0',
    gap: 12,
  },
  dateLine: {
    flex: 1,
    height: 1,
    background: '#1e293b',
  },
  dateLabel: {
    fontSize: '0.75rem',
    color: '#64748b',
    fontWeight: 500,
    whiteSpace: 'nowrap',
  },
  inputArea: {
    display: 'flex',
    gap: 8,
    padding: '12px 16px',
    borderTop: '1px solid #1e293b',
    background: '#0f172a',
    alignItems: 'flex-end',
  },
  messageInput: {
    flex: 1,
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 8,
    padding: '9px 14px',
    color: '#e2e8f0',
    fontSize: '1rem',
    resize: 'none',
    fontFamily: 'inherit',
    lineHeight: 1.5,
    minHeight: '44px',
    maxHeight: '160px',
    overflowY: 'hidden',
    transition: 'height 0.1s ease',
    boxSizing: 'border-box',
  },
  sendBtn: {
    background: '#3b82f6',
    color: '#fff',
    border: 'none',
    borderRadius: 8,
    padding: '0 20px',
    fontWeight: 600,
    cursor: 'pointer',
    fontSize: '0.9rem',
    transition: 'opacity 0.15s',
    height: '44px',
    boxSizing: 'border-box',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
  },
  scrollBtn: {
    position: 'absolute',
    bottom: 80,
    left: '50%',
    transform: 'translateX(-50%)',
    background: '#3b82f6',
    color: '#fff',
    border: 'none',
    borderRadius: 20,
    padding: '6px 16px',
    fontSize: '0.8rem',
    cursor: 'pointer',
    zIndex: 10,
    boxShadow: '0 2px 8px rgba(0,0,0,0.3)',
  },
  input: {
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 6,
    padding: '8px 12px',
    color: '#e2e8f0',
    fontSize: '1rem',
    width: '100%',
  },
  btnPrimary: {
    background: '#3b82f6',
    color: '#fff',
    border: 'none',
    borderRadius: 6,
    padding: '8px 16px',
    fontWeight: 600,
    cursor: 'pointer',
    fontSize: '0.85rem',
  },
  btnSecondary: {
    background: '#334155',
    color: '#e2e8f0',
    border: 'none',
    borderRadius: 6,
    padding: '8px 16px',
    cursor: 'pointer',
    fontSize: '0.85rem',
  },
  msgActions: {
    position: 'absolute',
    top: -4,
    right: 0,
    display: 'flex',
    gap: 2,
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 6,
    padding: '2px 4px',
    zIndex: 5,
    boxShadow: '0 2px 6px rgba(0,0,0,0.3)',
  },
  msgActionBtn: {
    background: 'none',
    border: 'none',
    color: '#94a3b8',
    cursor: 'pointer',
    padding: '2px 6px',
    fontSize: '0.8rem',
    borderRadius: 4,
    lineHeight: 1,
  },
  editInput: {
    width: '100%',
    background: '#0f172a',
    border: '1px solid #3b82f6',
    borderRadius: 6,
    padding: '6px 10px',
    color: '#e2e8f0',
    fontSize: '1rem',
    resize: 'none',
    fontFamily: 'inherit',
    lineHeight: 1.5,
  },
  editSaveBtn: {
    background: '#3b82f6',
    color: '#fff',
    border: 'none',
    borderRadius: 4,
    padding: '4px 12px',
    fontSize: '0.75rem',
    fontWeight: 600,
    cursor: 'pointer',
  },
  editCancelBtn: {
    background: '#334155',
    color: '#cbd5e1',
    border: 'none',
    borderRadius: 4,
    padding: '4px 12px',
    fontSize: '0.75rem',
    cursor: 'pointer',
  },
  replyPreview: {
    display: 'flex',
    gap: 8,
    padding: '6px 8px',
    marginBottom: 6,
    background: 'rgba(255,255,255,0.05)',
    borderRadius: 6,
    maxWidth: '100%',
    overflow: 'hidden',
  },
  replyBar: {
    display: 'flex',
    alignItems: 'center',
    gap: 10,
    padding: '8px 16px',
    background: '#1e293b',
    borderTop: '1px solid #334155',
  },
  replyCloseBtn: {
    background: 'none',
    border: 'none',
    color: '#64748b',
    cursor: 'pointer',
    fontSize: '0.9rem',
    padding: '2px 6px',
    flexShrink: 0,
  },
  typingIndicator: {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    padding: '4px 16px',
    fontSize: '0.75rem',
    color: '#64748b',
    fontStyle: 'italic',
    minHeight: 0,
  },
  typingDots: {
    display: 'inline-flex',
    gap: 1,
  },
  typingDot: {
    display: 'inline-block',
    animation: 'typingBounce 1.2s ease-in-out infinite',
    fontSize: '1rem',
    lineHeight: 1,
  },
  unreadBadge: {
    background: '#3b82f6',
    color: '#fff',
    fontSize: '0.7rem',
    fontWeight: 700,
    borderRadius: 10,
    padding: '1px 7px',
    minWidth: 18,
    textAlign: 'center',
    lineHeight: '16px',
    flexShrink: 0,
  },
  senderTypeToggle: {
    display: 'flex',
    gap: 0,
    marginBottom: 14,
    borderRadius: 8,
    overflow: 'hidden',
    border: '1px solid #334155',
  },
  toggleBtn: {
    flex: 1,
    padding: '10px 16px',
    border: 'none',
    cursor: 'pointer',
    fontSize: '0.9rem',
    fontWeight: 600,
    transition: 'background 0.15s, color 0.15s',
  },
  iconBtn: {
    background: 'none',
    border: '1px solid #334155',
    borderRadius: 6,
    color: '#e2e8f0',
    padding: '4px 10px',
    cursor: 'pointer',
    fontSize: '1.1rem',
    lineHeight: 1,
  },
  modalOverlay: {
    position: 'fixed',
    inset: 0,
    background: 'rgba(0,0,0,0.7)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 100,
  },
  modal: {
    background: '#1e293b',
    borderRadius: 12,
    padding: 28,
    width: '90%',
    maxWidth: 400,
    border: '1px solid #334155',
  },
  adminKeyBox: {
    display: 'flex',
    gap: 8,
    alignItems: 'center',
    background: '#0f172a',
    border: '1px solid #334155',
    borderRadius: 8,
    padding: 6,
  },
  adminKeyInput: {
    flex: 1,
    background: 'transparent',
    border: 'none',
    color: '#60a5fa',
    fontSize: '0.85rem',
    fontFamily: 'monospace',
    padding: '6px 8px',
    outline: 'none',
    minWidth: 0,
  },
  adminKeyCopyBtn: {
    background: '#334155',
    color: '#e2e8f0',
    border: 'none',
    borderRadius: 6,
    padding: '6px 14px',
    fontSize: '0.8rem',
    fontWeight: 600,
    cursor: 'pointer',
    whiteSpace: 'nowrap',
    transition: 'background 0.15s',
  },
  fileAttachBtn: {
    background: 'none',
    border: '1px solid #334155',
    borderRadius: 8,
    color: '#e2e8f0',
    padding: '0 12px',
    cursor: 'pointer',
    fontSize: '1.1rem',
    lineHeight: 1,
    flexShrink: 0,
    transition: 'background 0.15s',
    height: '44px',
    boxSizing: 'border-box',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  fileBubble: {
    padding: '10px 14px',
    lineHeight: 1.5,
    fontSize: '0.9rem',
  },
  fileImagePreview: {
    maxWidth: '100%',
    maxHeight: 200,
    borderRadius: 8,
    display: 'block',
    objectFit: 'contain',
  },
  fileInfo: {
    display: 'flex',
    alignItems: 'center',
    gap: 10,
  },
  fileIcon: {
    fontSize: '1.5rem',
    flexShrink: 0,
    lineHeight: 1,
  },
  fileDownloadBtn: {
    background: '#334155',
    color: '#e2e8f0',
    border: 'none',
    borderRadius: 6,
    padding: '4px 10px',
    cursor: 'pointer',
    fontSize: '0.85rem',
    textDecoration: 'none',
    flexShrink: 0,
    display: 'flex',
    alignItems: 'center',
  },
  participantPanel: {
    width: 240,
    minWidth: 240,
    borderLeft: '1px solid #1e293b',
    background: '#0f172a',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  participantHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '12px',
    borderBottom: '1px solid #1e293b',
  },
  participantList: {
    flex: 1,
    overflowY: 'auto',
  },
  participantItem: {
    padding: '8px 12px',
    borderBottom: '1px solid rgba(30,41,59,0.5)',
  },
  fileDeleteBtn: {
    background: 'none',
    border: 'none',
    color: '#ef4444',
    cursor: 'pointer',
    fontSize: '0.85rem',
    padding: '4px 6px',
    flexShrink: 0,
    lineHeight: 1,
  },
  searchPanel: {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  searchHeader: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '10px 16px',
    borderBottom: '1px solid #1e293b',
    background: '#0f172a',
  },
  searchInput: {
    flex: 1,
    background: 'transparent',
    border: 'none',
    color: '#e2e8f0',
    fontSize: '0.95rem',
    outline: 'none',
    fontFamily: 'inherit',
    padding: '4px 0',
  },
  searchCloseBtn: {
    background: '#334155',
    color: '#cbd5e1',
    border: 'none',
    borderRadius: 6,
    padding: '6px 14px',
    fontSize: '0.8rem',
    fontWeight: 600,
    cursor: 'pointer',
    flexShrink: 0,
  },
  searchResults: {
    flex: 1,
    overflowY: 'auto',
    padding: '4px 0',
  },
  searchResultItem: {
    padding: '10px 16px',
    cursor: 'pointer',
    borderBottom: '1px solid rgba(30,41,59,0.5)',
    transition: 'background 0.15s',
  },
};

// Inject global CSS (mobile styles, animations)
export function injectGlobalStyles() {
  if (typeof window === 'undefined') return;
  const style = document.createElement('style');
  style.textContent = `
    /* iOS Safari 100vh fix - fallback for browsers without dvh support */
    @supports not (height: 100dvh) {
      #root > div {
        height: 100vh !important;
        height: -webkit-fill-available !important;
        max-height: 100vh !important;
        max-height: -webkit-fill-available !important;
      }
    }
    @media (max-width: 768px) {
      .chat-mobile-header { display: flex !important; }
      .chat-content-header .chat-room-info { display: none !important; }
      .chat-content-header { padding: 6px 12px !important; justify-content: flex-end !important; border-bottom: 1px solid #1e293b !important; }
      .chat-header-actions { gap: 4px !important; }
      .chat-header-actions button { padding: 2px 6px !important; font-size: 0.85rem !important; }
      .chat-live-text { display: none !important; }
      .chat-live-indicator { margin-right: 2px; }
      .chat-sidebar {
        position: fixed !important;
        left: 0; top: 45px; bottom: 0;
        z-index: 50;
        width: 280px !important;
        min-width: 280px !important;
        background: #0f172a !important;
        box-shadow: 4px 0 24px rgba(0,0,0,0.5);
        animation: slideIn 0.2s ease-out;
      }
      .chat-sidebar-backdrop {
        display: block !important;
        position: fixed;
        inset: 0;
        top: 45px;
        background: rgba(0,0,0,0.5);
        z-index: 40;
      }
    }
    /* Prevent iOS bounce scroll on main layout */
    @media (max-width: 768px) {
      body { position: fixed; width: 100%; }
    }
    @media (min-width: 769px) {
      .chat-sidebar-backdrop { display: none !important; }
    }
    @media (max-width: 768px) {
      .participant-panel-wrapper {
        position: fixed !important;
        right: 0; top: 45px; bottom: 0;
        z-index: 50;
        width: 260px !important;
        min-width: 260px !important;
        box-shadow: -4px 0 24px rgba(0,0,0,0.5);
        animation: slideInRight 0.2s ease-out;
      }
    }
    @keyframes slideInRight {
      from { transform: translateX(100%); }
      to { transform: translateX(0); }
    }
    @keyframes slideIn {
      from { transform: translateX(-100%); }
      to { transform: translateX(0); }
    }
    @keyframes typingBounce {
      0%, 60%, 100% { opacity: 0.3; transform: translateY(0); }
      30% { opacity: 1; transform: translateY(-3px); }
    }
    /* Show bookmark star on room hover */
    .chat-sidebar > div > div:hover .room-bookmark-icon {
      opacity: 0.6 !important;
    }
    .room-bookmark-icon:hover {
      opacity: 1 !important;
    }
  `;
  document.head.appendChild(style);
  // Apply mobile header display dynamically
  const observer = new MutationObserver(() => {
    const mh = document.querySelector('[data-mobile-header]');
    if (mh && window.innerWidth <= 768) mh.style.display = 'flex';
  });
  observer.observe(document.body, { childList: true, subtree: true });
}
