import { useState } from 'react';
import type { WorkItem } from '../lib/types';
import { assignToAgent } from '../lib/api';
import { XIcon } from './icons';

interface Props {
  item: WorkItem;
  onClose: () => void;
}

const STORAGE_KEY = 'organiser:localPaths';

function loadPaths(): Record<string, string> {
  try { return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}'); } catch { return {}; }
}

function savePath(repo: string, path: string) {
  const paths = loadPaths();
  paths[repo] = path;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(paths));
}

export function AssignDialog({ item, onClose }: Props) {
  const repoKey = item.repo ?? item.url;
  const [localPath, setLocalPath] = useState(() => loadPaths()[repoKey] ?? '');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  async function handleAssign() {
    if (!localPath.trim()) { setError('Please enter the local repository path.'); return; }
    setLoading(true);
    setError('');
    try {
      await assignToAgent(
        item.account,
        item.repo ?? '',
        item.number ?? 0,
        item.title,
        item.url,
        localPath.trim(),
      );
      savePath(repoKey, localPath.trim());
      onClose();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()}>
        <button className="dialog-close" onClick={onClose}><XIcon /></button>
        <h2>Assign to Agent</h2>

        <div style={{ fontSize: 13, color: 'var(--muted)' }}>
          {item.repo && <span style={{ color: 'var(--text)' }}>{item.repo}</span>}
          {item.number && <span> #{item.number}</span>}
          <div style={{ marginTop: 4, color: 'var(--text)' }}>{item.title}</div>
        </div>

        <p style={{ fontSize: 12, color: 'var(--muted)', lineHeight: 1.6 }}>
          Review comments will be fetched and passed to Claude Code in a new Terminal window.
        </p>

        <div className="form-group">
          <label className="form-label">Local repository path</label>
          <input
            className="form-input"
            type="text"
            placeholder="/Users/you/projects/my-repo"
            value={localPath}
            onChange={e => setLocalPath(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleAssign()}
            autoFocus
          />
        </div>

        {error && <div className="form-error">{error}</div>}

        <div className="dialog-actions">
          <button className="btn" onClick={onClose}>Cancel</button>
          <button className="btn btn-primary" onClick={handleAssign} disabled={loading}>
            {loading ? 'Fetching comments…' : 'Open in Terminal'}
          </button>
        </div>
      </div>
    </div>
  );
}
