"use client";

import { ChevronRightIcon, InboxIcon, PlusIcon, ReplyIcon } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

import { EmptyState } from "@/components/layout/EmptyState";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { selectCurrentUser } from "@/features/auth/authSlice";
import { StatusBadge } from "@/features/tickets/components/StatusBadge";
import { isActive, ticketRef } from "@/features/tickets/labels";
import { useListTicketsQuery } from "@/features/tickets/ticketsApi";
import type { TicketSummary } from "@/features/tickets/types";
import { errorMessage } from "@/lib/api-error";
import { firstName, timeAgo } from "@/lib/format";
import { useAppSelector } from "@/lib/store/hooks";
import { cn } from "@/lib/utils";

function RequestRow({ ticket, framed = false }: { ticket: TicketSummary; framed?: boolean }) {
  return (
    <Link
      href={`/portal/requests/${ticket.number}`}
      className={cn(
        "grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 gap-y-1 px-4 py-3.5 hover:bg-muted/60 sm:px-5",
        framed ? "rounded-xl border bg-card" : "border-t first:border-t-0",
      )}
    >
      <span className="font-medium text-pretty">{ticket.subject}</span>
      <span className="justify-self-end">
        <StatusBadge status={ticket.status} audience="customer" />
      </span>
      <span className="flex flex-wrap gap-x-3 text-[13px] text-muted-foreground">
        <span className="font-mono text-xs text-faint">{ticketRef(ticket.number)}</span>
        Updated {timeAgo(ticket.updatedAt)}
      </span>
      <ChevronRightIcon aria-hidden className="size-4 justify-self-end text-faint" />
    </Link>
  );
}

export function PortalHome() {
  const user = useAppSelector(selectCurrentUser);
  const [filter, setFilter] = useState<"open" | "done">("open");
  // Customers have few requests; one page of the most recent covers them.
  const { data, isLoading, isError, error, refetch } = useListTicketsQuery({ sort: "recent", perPage: 100 });
  const all = data?.items ?? [];
  const needsReply = all.filter((t) => t.status === "pending");
  const open = all.filter((t) => isActive(t.status) && t.status !== "pending");
  const done = all.filter((t) => !isActive(t.status));
  const shown = filter === "open" ? open : done;

  return (
    <>
      <div className="flex flex-wrap items-end gap-x-6 gap-y-4">
        <div className="flex min-w-0 flex-1 basis-80 flex-col gap-1.5">
          <h1 className="text-2xl font-semibold tracking-tight text-balance">
            Hi {user ? firstName(user.name) : "there"}, how can we help?
          </h1>
          <p className="max-w-prose text-muted-foreground">Everything you ask us, and every reply from our team, is kept here.</p>
        </div>
        <Button size="lg" nativeButton={false} render={<Link href="/portal/new" />}>
          <PlusIcon />
          New request
        </Button>
      </div>

      {isError ? (
        <EmptyState
          icon={InboxIcon}
          title="We couldn't load your requests"
          action={
            <Button variant="outline" onClick={() => refetch()}>
              Try again
            </Button>
          }
        >
          {errorMessage(error)}
        </EmptyState>
      ) : isLoading ? (
        <div className="flex flex-col gap-2">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : all.length === 0 ? (
        <EmptyState
          icon={InboxIcon}
          title="No requests yet"
          action={
            <Button variant="outline" nativeButton={false} render={<Link href="/portal/new" />}>
              Send your first request
            </Button>
          }
        >
          When you contact support, your requests and our replies will appear here.
        </EmptyState>
      ) : (
        <>
          {needsReply.length > 0 && (
            <section aria-labelledby="needs-reply" className="flex flex-col gap-3 rounded-xl border border-l-[3px] border-l-warning bg-card p-4 sm:p-5">
              <h2 id="needs-reply" className="flex items-center gap-2 font-semibold text-warning">
                <ReplyIcon className="size-4" />
                {needsReply.length === 1 ? "1 request needs" : `${needsReply.length} requests need`} your reply
              </h2>
              <div className="flex flex-col gap-2">
                {needsReply.map((t) => (
                  <RequestRow key={t.number} ticket={t} framed />
                ))}
              </div>
            </section>
          )}

          <section className="flex flex-col gap-3">
            <div role="group" aria-label="Show requests" className="flex flex-wrap gap-1.5">
              {(
                [
                  ["open", "Open", open.length],
                  ["done", "Solved & closed", done.length],
                ] as const
              ).map(([value, label, count]) => (
                <button
                  key={value}
                  type="button"
                  aria-pressed={filter === value}
                  onClick={() => setFilter(value)}
                  className={cn(
                    "rounded-full border px-3.5 py-1 text-sm transition-colors",
                    filter === value
                      ? "border-foreground bg-foreground text-background"
                      : "bg-card text-muted-foreground hover:text-foreground",
                  )}
                >
                  {label}
                  <span className="ml-1.5 tabular-nums opacity-70">{count}</span>
                </button>
              ))}
            </div>
            {shown.length === 0 ? (
              <p className="rounded-xl border bg-card px-5 py-10 text-center text-sm text-muted-foreground">
                {filter === "open"
                  ? needsReply.length
                    ? "No other open requests."
                    : "No open requests. We're all caught up."
                  : "Solved requests will be listed here."}
              </p>
            ) : (
              <div className="overflow-hidden rounded-xl border bg-card">
                {shown.map((t) => (
                  <RequestRow key={t.number} ticket={t} />
                ))}
              </div>
            )}
          </section>
        </>
      )}
    </>
  );
}
