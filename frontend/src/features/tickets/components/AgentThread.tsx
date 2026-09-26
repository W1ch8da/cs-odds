import { LockIcon } from "lucide-react";

import { PersonAvatar } from "@/components/PersonAvatar";
import { formatDateTime, timeAgo } from "@/lib/format";
import { cn } from "@/lib/utils";

import { CHANNEL_LABEL, PRIORITY_LABEL, STATUS_LABEL } from "../labels";
import type { Priority, TicketDetail, TicketStatus, TimelineEvent } from "../types";

function Stamp({ iso }: { iso: string }) {
  return (
    <time dateTime={iso} title={formatDateTime(iso)} className="text-xs text-faint">
      {timeAgo(iso)}
    </time>
  );
}

/** One line describing a change, from the viewer's point of view. */
function describeEvent(e: TimelineEvent, meId?: string) {
  const isMe = e.actor?.id === meId;
  const who = e.actor ? (isMe ? "You" : e.actor.name) : "System";
  const strong = (text: string) => <span className="font-medium text-muted-foreground">{text}</span>;
  switch (e.kind) {
    case "status_changed":
      return <>{strong(who)} set status to {strong(STATUS_LABEL[e.newValue as TicketStatus] ?? e.newValue ?? "")}</>;
    case "priority_changed":
      return (
        <>
          {strong(who)} changed priority from {strong(PRIORITY_LABEL[e.oldValue as Priority] ?? "")} to{" "}
          {strong(PRIORITY_LABEL[e.newValue as Priority] ?? "")}
        </>
      );
    case "assignee_changed":
      if (!e.newValue) return <>{strong(who)} unassigned the ticket</>;
      if (e.newValue === e.actor?.name) return <>{strong(who)} took the ticket</>;
      return <>{strong(who)} assigned the ticket to {strong(e.newValue)}</>;
  }
}

type MessageProps = {
  name: string;
  /** Real name for the initials, when `name` is "You". */
  avatarName?: string;
  highlight: boolean;
  meta: string;
  createdAt: string;
  body: string;
  internal?: boolean;
};

function Message({ name, avatarName, highlight, meta, createdAt, body, internal = false }: MessageProps) {
  return (
    <article className="grid max-w-3xl grid-cols-[2rem_minmax(0,1fr)] gap-3">
      <PersonAvatar name={avatarName ?? name} highlight={highlight} className="size-8" />
      <div className={cn("rounded-xl border bg-card px-4 py-3", internal && "border-note-border bg-note")}>
        <div className="mb-1.5 flex flex-wrap items-baseline gap-x-2 gap-y-0.5 text-sm">
          <span className="font-semibold">{name}</span>
          {internal && (
            <span className="inline-flex items-center gap-1 text-xs font-medium text-warning">
              <LockIcon className="size-3" />
              Internal note
            </span>
          )}
          {meta && <span className="text-xs text-faint">{meta} ·</span>}
          <Stamp iso={createdAt} />
        </div>
        <p className="max-w-prose text-sm leading-relaxed break-words whitespace-pre-wrap">{body}</p>
      </div>
    </article>
  );
}

/** The ticket conversation as the team sees it: every message, note and change. */
export function AgentThread({ ticket, meId }: { ticket: TicketDetail; meId?: string }) {
  return (
    <div className="flex flex-col gap-4">
      <Message
        name={ticket.requester.name}
        highlight={false}
        meta={`via ${CHANNEL_LABEL[ticket.channel].toLowerCase()}`}
        createdAt={ticket.createdAt}
        body={ticket.description}
      />
      {ticket.timeline.map((item) =>
        item.type === "comment" ? (
          <Message
            key={item.id}
            name={item.author.id === meId ? "You" : item.author.name}
            avatarName={item.author.name}
            highlight={item.author.role !== "customer"}
            meta={item.internal ? "" : item.author.role === "customer" ? "replied" : "replied to the customer"}
            createdAt={item.createdAt}
            body={item.body}
            internal={item.internal}
          />
        ) : (
          <div key={item.id} className="flex items-center gap-3 pl-3 text-[13px] text-faint">
            <span aria-hidden className="size-2 shrink-0 rounded-full border-[1.5px] border-input bg-background" />
            <span className="ml-3">
              {describeEvent(item, meId)} · <Stamp iso={item.createdAt} />
            </span>
          </div>
        ),
      )}
    </div>
  );
}
