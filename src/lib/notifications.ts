import type { WorkItem, TaskState } from './types';

const NOTIFY_STATES = new Set<TaskState>(['approved', 'changes_requested']);

export interface StateChange {
  item: WorkItem;
  from: TaskState;
  to: TaskState;
}

export function diffPrStates(
  prev: WorkItem[],
  next: WorkItem[],
): StateChange[] {
  const prevMap = new Map(prev.map(i => [i.id, i.state]));
  const changes: StateChange[] = [];

  for (const item of next) {
    if (item.kind !== 'pr') continue;
    const prevState = prevMap.get(item.id);
    if (prevState === undefined) continue; // new PR, no notification on first fetch
    if (prevState !== item.state && NOTIFY_STATES.has(item.state)) {
      changes.push({ item, from: prevState, to: item.state });
    }
  }
  return changes;
}
