import { useState } from 'react';
import type { Account } from '../lib/types';
import { removeAccount } from '../lib/api';
import { AddAccountDialog } from './AddAccountDialog';
import { PlusIcon, TrashIcon, XIcon } from './icons';

interface Props {
  accounts: Account[];
  onClose: () => void;
  onChanged: () => void;
}

const KIND_LABEL: Record<string, string> = {
  github: 'GitHub.com',
  github_enterprise: 'GitHub Enterprise',
  gitlab: 'GitLab',
};

export function SettingsPanel({ accounts, onClose, onChanged }: Props) {
  const [showAdd, setShowAdd] = useState(false);
  const [removing, setRemoving] = useState<string | null>(null);

  async function handleRemove(id: string) {
    setRemoving(id);
    await removeAccount(id);
    onChanged();
    setRemoving(null);
  }

  return (
    <>
      <div className="dialog-overlay" onClick={onClose}>
        <div className="dialog" onClick={e => e.stopPropagation()}>
          <button className="dialog-close" onClick={onClose}><XIcon /></button>
          <h2>Accounts</h2>

          {accounts.length === 0 ? (
            <p style={{ color: 'var(--muted)', fontSize: 13 }}>
              No accounts yet. Add one to get started.
            </p>
          ) : (
            <div className="account-list">
              {accounts.map(a => (
                <div key={a.id} className="account-row">
                  <div className={`provider-dot ${a.kind}`} />
                  <div className="account-row-info">
                    <div className="account-row-label">{a.label}</div>
                    <div className="account-row-url">
                      {KIND_LABEL[a.kind]}
                      {a.baseUrl ? ` · ${a.baseUrl}` : ''}
                    </div>
                  </div>
                  <button
                    className="btn btn-sm btn-danger"
                    onClick={() => handleRemove(a.id)}
                    disabled={removing === a.id}
                    title="Remove account"
                  >
                    <TrashIcon />
                  </button>
                </div>
              ))}
            </div>
          )}

          <div className="dialog-actions" style={{ justifyContent: 'flex-start' }}>
            <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
              <PlusIcon /> Add account
            </button>
          </div>
        </div>
      </div>

      {showAdd && (
        <AddAccountDialog
          onClose={() => setShowAdd(false)}
          onAdded={onChanged}
        />
      )}
    </>
  );
}
