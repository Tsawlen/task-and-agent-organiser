import { useState, useEffect } from 'react';
import type { AgentSession } from '../lib/types';
import { getAgentSessions, dismissDoneSessions } from '../lib/api';
import { timeAgo } from '../lib/time';

const STATUS_DOT: Record<AgentSession['status'], string> = {
  working: '#3fb950',
  waiting: '#e3b341',
  done:    '#6e7681',
};

const STATUS_LABEL: Record<AgentSession['status'], string> = {
  working: 'Working',
  waiting: 'Waiting',
  done:    'Done',
};

function cwdBasename(cwd: string): string {
  return cwd.replace(/\/$/, '').split('/').pop() ?? cwd;
}

export function AgentsPage() {
  const [sessions, setSessions] = useState<AgentSession[]>([]);

  async function load() {
    try {
      const s = await getAgentSessions();
      s.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
      setSessions(s);
    } catch { /* app may be starting */ }
  }

  useEffect(() => {
    load();
    const id = setInterval(load, 5000);
    return () => clearInterval(id);
  }, []);

  const doneCount = sessions.filter(s => s.status === 'done').length;

  return (
    <div className="work-list">
      {sessions.length === 0 && (
        <div className="empty-state">
          <span>No active sessions.</span>
          <span style={{ fontSize: 12 }}>Sessions appear when Claude Code hooks fire.</span>
        </div>
      )}

      {sessions.map(s => (
        <div key={s.sessionId} className="work-item" style={{ cursor: 'default' }}>
          <span
            className="ci-dot"
            style={{ background: STATUS_DOT[s.status], flexShrink: 0 }}
            title={STATUS_LABEL[s.status]}
          />
          <div className="item-body">
            <div className="item-title">{cwdBasename(s.cwd)}</div>
            <div className="item-meta">
              <span className="item-repo" title={s.cwd}>{s.cwd}</span>
              {s.lastTool && <span className="item-number">{s.lastTool}</span>}
              <span className="item-time">{timeAgo(s.updatedAt)}</span>
            </div>
          </div>
          <span className="state-badge" style={{
            background: s.status === 'working' ? 'rgba(63,185,80,0.15)' :
                        s.status === 'waiting' ? 'rgba(227,179,65,0.15)' :
                        'rgba(110,118,129,0.2)',
            color: STATUS_DOT[s.status],
          }}>
            {STATUS_LABEL[s.status]}
          </span>
        </div>
      ))}

      {doneCount > 0 && (
        <div style={{ marginTop: 16, display: 'flex', justifyContent: 'flex-end' }}>
          <button
            className="btn btn-sm"
            onClick={async () => { await dismissDoneSessions(); load(); }}
          >
            Clear {doneCount} completed
          </button>
        </div>
      )}
    </div>
  );
}
