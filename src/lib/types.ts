export type HookSupport = 'supported' | 'file_required' | 'unsupported';

export interface AgentHookStatus {
  id: string;
  name: string;
  support: HookSupport;
  installed: boolean;
  configPath: string;
}

export type AgentStatus = 'working' | 'waiting' | 'done';

export interface AgentSession {
  sessionId: string;
  cwd: string;
  lastTool?: string;
  status: AgentStatus;
  updatedAt: string;
}

export type AccountKind = 'github' | 'github_enterprise' | 'gitlab';

export interface Account {
  id: string;
  kind: AccountKind;
  baseUrl: string;
  label: string;
  blockedOrgs: string[];
  selectedProjects: string[]; // GHE project node IDs
}

export interface GheProjectMeta {
  id: string;
  title: string;
  org: string;
}

export type TaskState =
  | 'open'
  | 'draft'
  | 'in_progress'
  | 'changes_requested'
  | 'approved'
  | 'merged'
  | 'closed';

export type WorkKind = 'issue' | 'pr' | 'project_item' | 'review_request';

export type CiStatus = 'success' | 'failure' | 'pending' | 'running';

export interface WorkItem {
  id: string;
  kind: WorkKind;
  title: string;
  url: string;
  updatedAt: string;
  state: TaskState;
  account: string;
  repo?: string;
  number?: number;
  project?: string;
  author?: string;
  ciStatus?: CiStatus;
}

export interface WorkList {
  accountId: string;
  items: WorkItem[];
  error?: string;
}
