export interface ChecklistItem {
  id: string;
  text: string;
}

export interface Checklist {
  id: string;
  name: string;
  items: ChecklistItem[];
}

/** Completion counters for one checklist. */
export interface ChecklistProgress {
  done: number;
  total: number;
}
