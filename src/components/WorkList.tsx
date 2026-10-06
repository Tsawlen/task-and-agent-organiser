import { openUrl } from '@tauri-apps/plugin-opener';
import type { Account, CiStatus, WorkItem, WorkList, TaskState, WorkKind } from '../lib/types';
import { timeAgo } from '../lib/time';
import {
  IssueIcon, PrIcon, DraftPrIcon, ProjectIcon, AlertIcon,
} from './icons';
import clsx from 'clsx';

interface Props {
  accounts: Account[];
  workLists: WorkList[];
  filter: string;
  view: 'issues' | 'prs' | 'reviews';
  activeAgents?: number;
  onAssign?: (item: WorkItem) => void;
}

const STATE_LABEL: Record<TaskState, string> = {
  open: 'Open',
  draft: 'Draft',
  in_progress: 'In Progress',
  changes_requested: 'Changes Requested',
  approved: 'Approved',
  merged: 'Merged',
  closed: 'Closed',
};

function ItemIcon({ kind, state }: { kind: WorkKind; state: TaskState }) {
  if (kind === 'project_item') return <span className="item-icon project"><ProjectIcon /></span>;
  if (kind === 'pr' || kind === 'review_request') {
    return state === 'draft'
      ? <span className="item-icon draft-pr"><DraftPrIcon /></span>
      : <span className="item-icon pr"><PrIcon /></span>;
  }
  return <span className="item-icon issue"><IssueIcon /></span>;
}

const CI_LABEL: Record<CiStatus, string> = {
  success: 'CI passed',
  failure: 'CI failed',
  pending: 'CI pending',
  running: 'CI running',
};

function CiDot({ status }: { status: CiStatus }) {
  return <span className={`ci-dot ci-${status}`} title={CI_LABEL[status]} />;
}

function WorkRow({ item, onAssign }: { item: WorkItem; onAssign?: (item: WorkItem) => void }) {
  function handleClick(e: React.MouseEvent) {
    e.preventDefault();
    openUrl(item.url).catch(() => window.open(item.url, '_blank'));
  }

  const showAssign = onAssign && item.state === 'changes_requested';

  return (
    <a
      className={clsx('work-item', item.state === 'changes_requested' && 'work-item-changes', item.state === 'approved' && 'work-item-approved')}
      href={item.url}
      onClick={handleClick}
    >
      <span className={clsx('state-badge', `state-${item.state}`)}>
        {STATE_LABEL[item.state]}
      </span>
      <ItemIcon kind={item.kind} state={item.state} />
      <div className="item-body">
        <div className="item-title">{item.title}</div>
        <div className="item-meta">
          {item.repo && <span className="item-repo">{item.repo}</span>}
          {item.number && <span className="item-number">#{item.number}</span>}
          {item.author && <span className="item-author">by {item.author}</span>}
          {item.project && <span className="item-project">{item.project}</span>}
          <span className="item-time">{timeAgo(item.updatedAt)}</span>
        </div>
      </div>
      {item.ciStatus && <CiDot status={item.ciStatus} />}
      {showAssign && (
        <button
          className="btn btn-sm assign-btn"
          title="Assign review comments to a Claude Code agent"
          onClick={e => { e.preventDefault(); e.stopPropagation(); onAssign(item); }}
        >
          Assign
        </button>
      )}
    </a>
  );
}

function Section({
  title, items, onAssign,
}: { title: string; items: WorkItem[]; onAssign?: (item: WorkItem) => void }) {
  if (items.length === 0) return null;
  return (
    <div className="section">
      <div className="section-header">
        {title}
        <span className="section-count">{items.length}</span>
      </div>
      {items.map(item => <WorkRow key={item.id} item={item} onAssign={onAssign} />)}
    </div>
  );
}

export function WorkList({ accounts, workLists, filter, view, onAssign }: Props) {
  const accountMap = Object.fromEntries(accounts.map(a => [a.id, a]));

  const filteredLists = workLists.filter(wl => {
    if (filter === 'all') return true;
    const acc = accountMap[wl.accountId];
    return acc?.kind === filter;
  });

  if (filteredLists.length === 0) {
    return (
      <div className="work-list">
        <div className="empty-state">
          <span>No accounts configured.</span>
          <span style={{ fontSize: 12 }}>Open Settings to add a GitHub or GitLab account.</span>
        </div>
      </div>
    );
  }

  const allItems = filteredLists.flatMap(wl => wl.items);
  const visibleItems = view === 'issues'
    ? allItems.filter(i => i.kind === 'issue' || i.kind === 'project_item')
    : view === 'prs'
    ? allItems.filter(i => i.kind === 'pr')
    : allItems.filter(i => i.kind === 'review_request');

  const errors = filteredLists.filter(wl => wl.error);

  if (visibleItems.length === 0 && errors.length === 0) {
    return (
      <div className="work-list">
        <div className="empty-state">
          <span>All clear — nothing here.</span>
        </div>
      </div>
    );
  }

  if (view === 'issues') {
    const issues = visibleItems.filter(i => i.kind === 'issue');
    const projectItems = visibleItems.filter(i => i.kind === 'project_item');
    return (
      <div className="work-list">
        {errors.map(wl => (
          <div key={wl.accountId} className="error-banner" style={{ marginTop: 16 }}>
            <AlertIcon />
            <span><strong>{accountMap[wl.accountId]?.label ?? wl.accountId}:</strong> {wl.error}</span>
          </div>
        ))}
        <Section title="Assigned Issues" items={issues} onAssign={onAssign} />
        <Section title="Project Items" items={projectItems} onAssign={onAssign} />
      </div>
    );
  }

  return (
    <div className="work-list">
      {errors.map(wl => (
        <div key={wl.accountId} className="error-banner" style={{ marginTop: 16 }}>
          <AlertIcon />
          <span><strong>{accountMap[wl.accountId]?.label ?? wl.accountId}:</strong> {wl.error}</span>
        </div>
      ))}
      {view === 'prs' && <Section title="My Pull / Merge Requests" items={visibleItems} onAssign={onAssign} />}
      {view === 'reviews' && <Section title="Review Requests" items={visibleItems} onAssign={onAssign} />}
    </div>
  );
}
