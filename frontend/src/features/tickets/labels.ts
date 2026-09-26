import type { Channel, ListTicketsParams, Priority, TicketStatus } from "./types";

/** Wording for the support team. */
export const STATUS_LABEL: Record<TicketStatus, string> = {
  open: "Open",
  pending: "Pending",
  on_hold: "On hold",
  solved: "Solved",
  closed: "Closed",
};

export const STATUS_HINT: Record<TicketStatus, string> = {
  open: "Needs a reply from your team",
  pending: "Waiting on the customer",
  on_hold: "Waiting on a third party",
  solved: "Resolved. Reopens if the customer replies",
  closed: "Archived. Can't be changed",
};

/** Wording for customers. "On hold" is internal detail, so it reads as in progress. */
export const CUSTOMER_STATUS_LABEL: Record<TicketStatus, string> = {
  open: "Waiting on support",
  on_hold: "In progress",
  pending: "Needs your reply",
  solved: "Solved",
  closed: "Closed",
};

/** Semantic colour for each status, shared by both audiences. */
export const STATUS_TONE: Record<TicketStatus, string> = {
  open: "bg-info-soft text-info",
  pending: "bg-warning-soft text-warning",
  on_hold: "bg-muted text-muted-foreground",
  solved: "bg-success-soft text-success",
  closed: "bg-muted text-faint",
};

export const PRIORITY_LABEL: Record<Priority, string> = {
  urgent: "Urgent",
  high: "High",
  normal: "Normal",
  low: "Low",
};

export const PRIORITIES: Priority[] = ["urgent", "high", "normal", "low"];
export const STATUSES: TicketStatus[] = ["open", "pending", "on_hold", "solved", "closed"];

export const CHANNEL_LABEL: Record<Channel, string> = {
  portal: "Portal",
  email: "Email",
  agent: "Created by the team",
};

/** Customers describe impact; it maps to a priority. */
export const IMPACT_OPTIONS: { priority: Priority; title: string; description: string }[] = [
  { priority: "low", title: "A question", description: "How-to, account or billing question" },
  { priority: "normal", title: "Something isn't working", description: "There's a workaround for now" },
  { priority: "high", title: "It's blocking my work", description: "I can't do an important task" },
  { priority: "urgent", title: "Our team is stopped", description: "Many people are affected right now" },
];

const ACTIVE = "open,pending,on_hold";

export type QueueView = "mine" | "unassigned" | "open" | "solved";

export const QUEUE_VIEWS: Record<QueueView, { label: string; params: ListTicketsParams }> = {
  mine: { label: "My open tickets", params: { status: ACTIVE, assignee: "me" } },
  unassigned: { label: "Unassigned", params: { status: ACTIVE, assignee: "unassigned" } },
  open: { label: "All open", params: { status: ACTIVE } },
  solved: { label: "Solved", params: { status: "solved,closed", sort: "recent" } },
};

export function isQueueView(value: unknown): value is QueueView {
  return typeof value === "string" && value in QUEUE_VIEWS;
}

export function ticketRef(number: number): string {
  return `TKT-${number}`;
}

export function isActive(status: TicketStatus): boolean {
  return status === "open" || status === "pending" || status === "on_hold";
}
