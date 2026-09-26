"use client";

import {
  ArrowLeftIcon,
  CheckCircle2Icon,
  ClockIcon,
  InboxIcon,
  Loader2Icon,
  ReplyIcon,
  SearchXIcon,
  SparklesIcon,
  type LucideIcon,
} from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { toast } from "sonner";

import { EmptyState } from "@/components/layout/EmptyState";
import { PersonAvatar } from "@/components/PersonAvatar";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { AttachButton, AttachmentList, UploadList } from "@/features/attachments/components";
import type { Attachment } from "@/features/attachments/types";
import { useUploads } from "@/features/attachments/useUploads";
import { selectCurrentUser } from "@/features/auth/authSlice";
import { StatusBadge } from "@/features/tickets/components/StatusBadge";
import { CUSTOMER_STATUS_LABEL, isActive, ticketRef } from "@/features/tickets/labels";
import { useAddCommentMutation, useGetTicketQuery, useUpdateTicketMutation } from "@/features/tickets/ticketsApi";
import type { TicketDetail, TicketStatus, TimelineEvent } from "@/features/tickets/types";
import { errorMessage, errorStatus } from "@/lib/api-error";
import { firstName, formatDate, formatDateTime, timeAgo } from "@/lib/format";
import { useAppSelector } from "@/lib/store/hooks";
import { cn } from "@/lib/utils";

function lastSupportReply(ticket: TicketDetail) {
  return [...ticket.timeline].reverse().find((i) => i.type === "comment" && i.author.role !== "customer");
}

function Banner({ ticket, fresh }: { ticket: TicketDetail; fresh: boolean }) {
  const agent = lastSupportReply(ticket);
  const agentName = agent?.type === "comment" ? firstName(agent.author.name) : "Our team";
  let tone = "border bg-card";
  let Icon: LucideIcon = ClockIcon;
  let iconTone = "text-info";
  let title: string;
  let body: React.ReactNode;

  if (fresh) {
    [tone, Icon, iconTone] = ["bg-accent", SparklesIcon, "text-accent-foreground"];
    title = `We've got your request, ${ticketRef(ticket.number)}`;
    body = `We've emailed a confirmation to ${ticket.requester.email}. Our team will reply here and by email.`;
  } else {
    switch (ticket.status) {
      case "pending":
        [tone, Icon, iconTone] = ["bg-warning-soft", ReplyIcon, "text-warning"];
        title = `${agentName} is waiting for your reply`;
        body = "Answer below so we can keep going.";
        break;
      case "open":
        title = "With our support team";
        body = "We'll reply here as soon as we can, and email you when we do.";
        break;
      case "on_hold":
        title = "We're working on it";
        body = `${agentName} is following up with another team and will update you here.`;
        break;
      case "solved":
        [tone, Icon, iconTone] = ["bg-success-soft", CheckCircle2Icon, "text-success"];
        title = "This request is solved";
        body = "Still having trouble? Reply below and we'll reopen it.";
        break;
      default:
        [Icon, iconTone] = [InboxIcon, "text-muted-foreground"];
        title = "This request is closed";
        body = (
          <>
            Need more help?{" "}
            <Link href="/portal/new" className="font-medium text-accent-foreground underline">
              Start a new request
            </Link>
            .
          </>
        );
    }
  }

  return (
    <div className={cn("flex items-start gap-3 rounded-xl px-4 py-3.5", tone)}>
      <Icon className={cn("mt-0.5 size-[18px] shrink-0", iconTone)} />
      <div className="text-[15px]">
        <p className="font-semibold">{title}</p>
        <p className="text-muted-foreground">{body}</p>
      </div>
    </div>
  );
}

function eventText(e: TimelineEvent, meId?: string) {
  const mine = e.actor?.id === meId;
  const to = e.newValue as TicketStatus;
  if (to === "solved") return mine ? "You marked this request as solved" : `${e.actor ? firstName(e.actor.name) : "We"} marked this request as solved`;
  if (to === "open" && (e.oldValue === "solved" || e.oldValue === "pending") && mine) {
    return e.oldValue === "solved" ? "Request reopened" : null;
  }
  if (to === "closed") return "Request closed";
  if (mine) return null;
  return `Status changed to "${CUSTOMER_STATUS_LABEL[to] ?? to}"`;
}

function Stamp({ iso }: { iso: string }) {
  return (
    <time dateTime={iso} title={formatDateTime(iso)} className="text-[13px] text-faint">
      {timeAgo(iso)}
    </time>
  );
}

type BubbleProps = { name: string; avatarName: string; staff: boolean; iso: string; body: string; attachments: Attachment[] };

function Bubble({ name, avatarName, staff, iso, body, attachments }: BubbleProps) {
  return (
    <article className="grid grid-cols-[2rem_minmax(0,1fr)] gap-3">
      <PersonAvatar name={avatarName} highlight={staff} className="size-8" />
      <div className={cn("rounded-xl border px-4 py-3", staff ? "bg-card" : "bg-muted/60")}>
        <div className="mb-1 flex flex-wrap items-baseline gap-x-2 text-sm">
          <span className="font-semibold">{name}</span>
          {staff && <span className="text-xs font-medium text-accent-foreground">Support team</span>}
          <Stamp iso={iso} />
        </div>
        <p className="max-w-prose break-words whitespace-pre-wrap">{body}</p>
        <AttachmentList attachments={attachments} className="mt-3" />
      </div>
    </article>
  );
}

/** The conversation as the customer sees it: public replies and key status changes. */
function Thread({ ticket, meId }: { ticket: TicketDetail; meId?: string }) {
  return (
    <div className="flex flex-col gap-4">
      <Bubble
        name="You"
        avatarName={ticket.requester.name}
        staff={false}
        iso={ticket.createdAt}
        body={ticket.description}
        attachments={ticket.attachments}
      />
      {ticket.timeline.map((item) => {
        if (item.type === "comment") {
          const mine = item.author.id === meId;
          return (
            <Bubble
              key={item.id}
              name={mine ? "You" : item.author.name}
              avatarName={item.author.name}
              staff={item.author.role !== "customer"}
              iso={item.createdAt}
              body={item.body}
              attachments={item.attachments}
            />
          );
        }
        const text = eventText(item, meId);
        return (
          text && (
            <p key={item.id} className="flex items-center gap-3 pl-3 text-[13px] text-faint">
              <span aria-hidden className="size-2 rounded-full border-[1.5px] border-input" />
              <span className="ml-3">
                {text} · <Stamp iso={item.createdAt} />
              </span>
            </p>
          )
        );
      })}
    </div>
  );
}

function ReplyBox({ ticket }: { ticket: TicketDetail }) {
  const [body, setBody] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [addComment, { isLoading }] = useAddCommentMutation();
  const files = useUploads();
  const reopening = ticket.status === "solved";

  const send = async () => {
    if (files.busy) {
      setError("Wait for your files to finish uploading.");
      return;
    }
    if (files.failed) {
      setError("Remove the files that couldn't be uploaded, then send.");
      return;
    }
    if (!body.trim()) {
      setError("Write a reply before sending.");
      return;
    }
    setError(null);
    try {
      await addComment({ number: ticket.number, body, attachmentIds: files.attachmentIds }).unwrap();
      setBody("");
      files.reset();
      toast.success(reopening ? `${ticketRef(ticket.number)} reopened. We'll get back to you soon.` : "Reply sent");
    } catch (err) {
      setError(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  };

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        void send();
      }}
      className="flex flex-col gap-2.5 rounded-xl border bg-card p-3.5"
    >
      <label htmlFor="reply" className="text-xs font-medium tracking-wider text-faint uppercase">
        {reopening ? "Reply to reopen" : "Your reply"}
      </label>
      <textarea
        id="reply"
        value={body}
        onChange={(e) => setBody(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            void send();
          }
        }}
        aria-invalid={error ? true : undefined}
        placeholder="Write your reply…"
        className="min-h-28 w-full resize-y rounded-lg border bg-card px-3 py-2 outline-none placeholder:text-faint focus:border-ring"
      />
      <UploadList uploads={files.uploads} onRemove={files.remove} />
      {error && (
        <p role="alert" className="text-xs text-destructive">
          {error}
        </p>
      )}
      <div className="flex flex-wrap items-center gap-2">
        <AttachButton onFiles={files.add} />
        <span className="text-[13px] text-faint">Your reply goes straight to the support team.</span>
        <Button type="submit" className="ml-auto" disabled={isLoading || files.busy}>
          {isLoading && <Loader2Icon className="animate-spin" />}
          {reopening ? "Reopen and send" : "Send reply"}
        </Button>
      </div>
    </form>
  );
}

function MarkSolved({ ticket }: { ticket: TicketDetail }) {
  const [confirming, setConfirming] = useState(false);
  const [update, { isLoading }] = useUpdateTicketMutation();

  const solve = async () => {
    try {
      await update({ number: ticket.number, status: "solved" }).unwrap();
      toast.success(`${ticketRef(ticket.number)} marked as solved. Thanks for letting us know.`);
    } catch (err) {
      toast.error(errorMessage(err as Parameters<typeof errorMessage>[0]));
    } finally {
      setConfirming(false);
    }
  };

  return (
    <div className="flex flex-col gap-2.5 rounded-xl border bg-card p-4 text-sm">
      {confirming ? (
        <>
          <p className="text-muted-foreground">Close this request as solved? You can reopen it later by replying.</p>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" onClick={solve} disabled={isLoading} autoFocus>
              Yes, it&apos;s solved
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setConfirming(false)}>
              Cancel
            </Button>
          </div>
        </>
      ) : (
        <>
          <p className="text-muted-foreground">Everything working now?</p>
          <Button size="sm" variant="outline" className="self-start" onClick={() => setConfirming(true)}>
            <CheckCircle2Icon />
            Mark as solved
          </Button>
        </>
      )}
    </div>
  );
}

export function RequestConversation({ number, fresh }: { number: number; fresh: boolean }) {
  const me = useAppSelector(selectCurrentUser);
  const { data: ticket, isLoading, isError, error, refetch } = useGetTicketQuery(number);
  const back = (
    <Link href="/portal" className="inline-flex items-center gap-1.5 self-start text-sm text-muted-foreground hover:text-foreground">
      <ArrowLeftIcon className="size-4" />
      My requests
    </Link>
  );

  if (isError) {
    const missing = errorStatus(error) === 404;
    return (
      <>
        {back}
        <EmptyState
          icon={SearchXIcon}
          title={missing ? "We couldn't find that request" : "We couldn't load this request"}
          action={
            !missing && (
              <Button variant="outline" onClick={() => refetch()}>
                Try again
              </Button>
            )
          }
        >
          {missing ? "It may belong to another account. Check the link, or go back to your requests." : errorMessage(error)}
        </EmptyState>
      </>
    );
  }

  if (isLoading || !ticket) {
    return (
      <>
        {back}
        <Skeleton className="h-8 w-2/3" />
        <Skeleton className="h-28 w-full" />
      </>
    );
  }

  return (
    <>
      <div className="flex flex-col gap-2">
        {back}
        <h1 className="text-2xl font-semibold tracking-tight text-balance">{ticket.subject}</h1>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
          <StatusBadge status={ticket.status} audience="customer" />
          <span className="font-mono text-xs text-faint">{ticketRef(ticket.number)}</span>
          <span>Opened {formatDate(ticket.createdAt)}</span>
        </div>
      </div>

      <div className="grid items-start gap-7 lg:grid-cols-[minmax(0,1fr)_16rem]">
        <div className="flex min-w-0 flex-col gap-5">
          <Banner ticket={ticket} fresh={fresh && ticket.timeline.length === 0} />
          <Thread ticket={ticket} meId={me?.id} />
          {ticket.status !== "closed" && <ReplyBox ticket={ticket} />}
        </div>
        <aside className="flex flex-col gap-4">
          <div className="rounded-xl border bg-card p-4">
            <h2 className="mb-3 text-sm font-semibold">Request details</h2>
            <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2 text-sm">
              <dt className="text-muted-foreground">Number</dt>
              <dd className="font-mono text-xs leading-5">{ticketRef(ticket.number)}</dd>
              <dt className="text-muted-foreground">Opened</dt>
              <dd>{formatDate(ticket.createdAt)}</dd>
              <dt className="text-muted-foreground">Contact</dt>
              <dd className="break-all">{ticket.requester.email}</dd>
            </dl>
          </div>
          {isActive(ticket.status) && <MarkSolved ticket={ticket} />}
        </aside>
      </div>
    </>
  );
}
