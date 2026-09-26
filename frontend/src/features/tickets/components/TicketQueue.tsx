"use client";

import { ChevronLeftIcon, ChevronRightIcon, GlobeIcon, HeadsetIcon, InboxIcon, MailIcon, PlusIcon, SearchIcon } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";

import { EmptyState } from "@/components/layout/EmptyState";
import { PageHeader } from "@/components/layout/PageHeader";
import { PersonAvatar } from "@/components/PersonAvatar";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { selectCurrentUser } from "@/features/auth/authSlice";
import { errorMessage } from "@/lib/api-error";
import { timeAgo } from "@/lib/format";
import { useAppDispatch, useAppSelector } from "@/lib/store/hooks";
import { cn } from "@/lib/utils";

import { CHANNEL_LABEL, PRIORITIES, PRIORITY_LABEL, QUEUE_VIEWS, type QueueView, ticketRef } from "../labels";
import { useListTicketsQuery } from "../ticketsApi";
import { pageChanged, priorityFilterChanged, searchChanged, selectQueue, viewOpened } from "../ticketsSlice";
import type { Channel, Priority, TicketSummary } from "../types";
import { PriorityLabel } from "./PriorityLabel";
import { StatusBadge } from "./StatusBadge";

const PER_PAGE = 25;
const CHANNEL_ICON: Record<Channel, typeof MailIcon> = { email: MailIcon, portal: GlobeIcon, agent: HeadsetIcon };
// Written out in full: Tailwind only generates classes it can find in the source.
const ROW_GRID_MD = "md:grid md:grid-cols-[minmax(0,1fr)_7rem_6rem_10rem_5.5rem] md:items-center md:gap-3";

function SearchBox() {
  const dispatch = useAppDispatch();
  const { search } = useAppSelector(selectQueue);
  const [value, setValue] = useState(search);

  // Wait for a pause in typing before querying.
  useEffect(() => {
    const id = setTimeout(() => {
      if (value !== search) dispatch(searchChanged(value));
    }, 300);
    return () => clearTimeout(id);
  }, [value, search, dispatch]);

  return (
    <label className="flex h-9 min-w-0 flex-1 basis-60 items-center gap-2 rounded-lg border bg-card px-3 text-faint focus-within:border-ring sm:max-w-sm">
      <SearchIcon className="size-4 shrink-0" />
      <input
        id="ticket-search"
        type="search"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder="Search subject, customer or TKT-number"
        aria-label="Search tickets"
        autoComplete="off"
        className="w-full bg-transparent text-sm text-foreground outline-none placeholder:text-faint"
      />
    </label>
  );
}

function PriorityChips() {
  const dispatch = useAppDispatch();
  const { priority } = useAppSelector(selectQueue);
  const options: (Priority | "all")[] = ["all", ...PRIORITIES];
  return (
    <div role="group" aria-label="Filter by priority" className="flex flex-wrap gap-1.5">
      {options.map((p) => (
        <button
          key={p}
          type="button"
          aria-pressed={priority === p}
          onClick={() => dispatch(priorityFilterChanged(p))}
          className={cn(
            "rounded-full border px-3 py-1 text-sm transition-colors",
            priority === p
              ? "border-foreground bg-foreground text-background"
              : "bg-card text-muted-foreground hover:border-input hover:text-foreground",
          )}
        >
          {p === "all" ? "All priorities" : PRIORITY_LABEL[p]}
        </button>
      ))}
    </div>
  );
}

function TicketRow({ ticket, selected, meId, onOpen }: { ticket: TicketSummary; selected: boolean; meId?: string; onOpen: () => void }) {
  const ChannelIcon = CHANNEL_ICON[ticket.channel];
  const isMe = ticket.assignee?.id === meId;
  return (
    <Link
      href={`/agent/tickets/${ticket.number}`}
      onClick={(e) => {
        e.preventDefault();
        onOpen();
      }}
      aria-current={selected ? "true" : undefined}
      className={cn(
        "border-t border-l-[3px] border-l-transparent px-4 py-2.5 outline-none first:border-t-0 hover:bg-muted/60 focus-visible:bg-accent",
        "max-md:flex max-md:flex-col max-md:gap-2",
        ROW_GRID_MD,
        selected && "bg-accent hover:bg-accent",
        ticket.priority === "urgent" && "border-l-destructive",
        ticket.priority === "high" && "border-l-warning",
      )}
    >
      <div className="flex min-w-0 flex-col gap-0.5">
        <div className="flex min-w-0 items-baseline gap-2">
          <span className="shrink-0 font-mono text-xs text-faint">{ticketRef(ticket.number)}</span>
          <span className="truncate font-medium">{ticket.subject}</span>
        </div>
        <div className="flex min-w-0 items-center gap-1.5 text-[13px] text-muted-foreground">
          <span className="truncate">{ticket.requester.name}</span>
          <ChannelIcon className="size-3.5 shrink-0" aria-label={CHANNEL_LABEL[ticket.channel]} />
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 md:contents">
        <div>
          <StatusBadge status={ticket.status} />
        </div>
        <PriorityLabel priority={ticket.priority} />
        <div className="flex min-w-0 items-center gap-2 text-sm text-muted-foreground">
          {ticket.assignee ? (
            <>
              <PersonAvatar name={ticket.assignee.name} highlight={isMe} className="size-6" />
              <span className="truncate">{isMe ? "You" : ticket.assignee.name}</span>
            </>
          ) : (
            <span className="text-faint italic">Unassigned</span>
          )}
        </div>
        <div className="text-[13px] text-faint tabular-nums md:text-right">{timeAgo(ticket.updatedAt)}</div>
      </div>
    </Link>
  );
}

export function TicketQueue({ view }: { view: QueueView }) {
  const router = useRouter();
  const dispatch = useAppDispatch();
  const me = useAppSelector(selectCurrentUser);
  const { priority, search, page } = useAppSelector(selectQueue);
  const [selected, setSelected] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dispatch(viewOpened(view));
  }, [dispatch, view]);

  const { label, params } = QUEUE_VIEWS[view];
  const { data, isLoading, isFetching, isError, error, refetch } = useListTicketsQuery({
    ...params,
    priority: priority === "all" ? undefined : priority,
    q: search || undefined,
    page,
    perPage: PER_PAGE,
  });
  const tickets = data?.items ?? [];
  const selectedIndex = Math.min(selected, Math.max(tickets.length - 1, 0));

  const open = (ticket: TicketSummary) => router.push(`/agent/tickets/${ticket.number}`);

  // J/K to move, Enter to open, / to search. Ignored while typing.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (/INPUT|TEXTAREA|SELECT/.test(target.tagName) || target.isContentEditable || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "/") {
        e.preventDefault();
        document.getElementById("ticket-search")?.focus();
      } else if ((e.key === "j" || e.key === "ArrowDown") && tickets.length) {
        e.preventDefault();
        setSelected(Math.min(selectedIndex + 1, tickets.length - 1));
      } else if ((e.key === "k" || e.key === "ArrowUp") && tickets.length) {
        e.preventDefault();
        setSelected(Math.max(selectedIndex - 1, 0));
      } else if (e.key === "Enter" && tickets[selectedIndex]) {
        open(tickets[selectedIndex]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  useEffect(() => {
    listRef.current?.querySelector('[aria-current="true"]')?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const total = data?.total ?? 0;
  const first = total === 0 ? 0 : (page - 1) * PER_PAGE + 1;
  const last = Math.min(page * PER_PAGE, total);
  const filtered = priority !== "all" || search !== "";

  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title={label}
        description={
          view === "solved" ? "Most recently updated first." : "Most urgent first, then oldest first."
        }
        actions={
          <Button nativeButton={false} render={<Link href="/agent/tickets/new" />}>
            <PlusIcon />
            New ticket
          </Button>
        }
      />

      <div className="flex flex-wrap items-center gap-2">
        <SearchBox />
        <PriorityChips />
      </div>

      {isError ? (
        <EmptyState
          icon={InboxIcon}
          title="We couldn't load tickets"
          action={
            <Button variant="outline" onClick={() => refetch()}>
              Try again
            </Button>
          }
        >
          {errorMessage(error)}
        </EmptyState>
      ) : !isLoading && tickets.length === 0 ? (
        <EmptyState icon={InboxIcon} title={filtered ? "No tickets match these filters" : "Nothing here"}>
          {filtered
            ? "Try another priority or search term."
            : view === "mine"
              ? "You have no open tickets. Check Unassigned for new requests."
              : "This view is clear."}
        </EmptyState>
      ) : (
        <div className={cn("overflow-hidden rounded-xl border bg-card transition-opacity", isFetching && !isLoading && "opacity-70")}>
          <div className={cn("hidden border-b bg-muted/60 px-4 py-2 pl-[19px] text-xs font-medium text-faint", ROW_GRID_MD)}>
            <span>Ticket</span>
            <span>Status</span>
            <span>Priority</span>
            <span>Assignee</span>
            <span className="text-right">Updated</span>
          </div>
          <div ref={listRef} className="flex flex-col" aria-busy={isFetching}>
            {isLoading
              ? Array.from({ length: 5 }, (_, i) => (
                  <div key={i} className="border-t px-4 py-3 first:border-t-0">
                    <Skeleton className="h-10 w-full" />
                  </div>
                ))
              : tickets.map((t, i) => (
                  <TicketRow key={t.number} ticket={t} selected={i === selectedIndex} meId={me?.id} onOpen={() => open(t)} />
                ))}
          </div>
        </div>
      )}

      {total > 0 && (
        <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
          <span className="tabular-nums">
            {first}–{last} of {total}
          </span>
          <span className="hidden text-xs text-faint lg:inline">
            <kbd className="rounded border px-1 font-mono">J</kbd> <kbd className="rounded border px-1 font-mono">K</kbd> move ·{" "}
            <kbd className="rounded border px-1 font-mono">Enter</kbd> open · <kbd className="rounded border px-1 font-mono">/</kbd>{" "}
            search
          </span>
          <div className="ml-auto flex gap-1">
            <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => dispatch(pageChanged(page - 1))}>
              <ChevronLeftIcon />
              Previous
            </Button>
            <Button variant="outline" size="sm" disabled={last >= total} onClick={() => dispatch(pageChanged(page + 1))}>
              Next
              <ChevronRightIcon />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
