import type { Role } from "@/features/auth/types";

export type TicketStatus = "open" | "pending" | "on_hold" | "solved" | "closed";
export type Priority = "low" | "normal" | "high" | "urgent";
export type Channel = "portal" | "email" | "agent";

export type UserRef = { id: string; name: string; email: string; role: Role };

export type TicketSummary = {
  number: number;
  subject: string;
  status: TicketStatus;
  priority: Priority;
  channel: Channel;
  requester: UserRef;
  assignee: UserRef | null;
  createdAt: string;
  updatedAt: string;
};

export type TimelineComment = {
  type: "comment";
  id: string;
  author: UserRef;
  body: string;
  internal: boolean;
  createdAt: string;
};

export type TimelineEvent = {
  type: "event";
  id: string;
  actor: UserRef | null;
  kind: "status_changed" | "priority_changed" | "assignee_changed";
  /** Status/priority codes, or display names for assignee changes. */
  oldValue: string | null;
  newValue: string | null;
  createdAt: string;
};

export type TimelineItem = TimelineComment | TimelineEvent;

export type TicketDetail = TicketSummary & {
  description: string;
  resolvedAt: string | null;
  timeline: TimelineItem[];
  requesterTicketCount: number;
};

export type Page<T> = { items: T[]; total: number; page: number; perPage: number };

export type ViewCounts = { mine: number; unassigned: number; open: number; solved: number };

export type ListTicketsParams = {
  status?: string;
  priority?: Priority;
  assignee?: "me" | "unassigned" | string;
  q?: string;
  sort?: "priority" | "recent";
  page?: number;
  perPage?: number;
};

export type CreateTicketRequest = {
  subject: string;
  description: string;
  priority?: Priority;
  requesterEmail?: string;
  requesterName?: string;
  assigneeId?: string;
};

export type UpdateTicketRequest = {
  number: number;
  status?: TicketStatus;
  priority?: Priority;
  /** `null` unassigns. */
  assigneeId?: string | null;
};

export type AddCommentRequest = {
  number: number;
  body: string;
  internal?: boolean;
  status?: TicketStatus;
};
