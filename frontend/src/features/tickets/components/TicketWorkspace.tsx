"use client";

import { ArrowLeftIcon, CheckCircle2Icon, SearchXIcon } from "lucide-react";
import Link from "next/link";
import { toast } from "sonner";

import { EmptyState } from "@/components/layout/EmptyState";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { selectCurrentUser } from "@/features/auth/authSlice";
import { errorMessage, errorStatus } from "@/lib/api-error";
import { useAppSelector } from "@/lib/store/hooks";

import { QUEUE_VIEWS, isActive, ticketRef } from "../labels";
import { useGetTicketQuery, useUpdateTicketMutation } from "../ticketsApi";
import { selectQueue } from "../ticketsSlice";
import { AgentComposer } from "./AgentComposer";
import { AgentThread } from "./AgentThread";
import { StatusBadge } from "./StatusBadge";
import { TicketProperties } from "./TicketProperties";

export function TicketWorkspace({ number }: { number: number }) {
  const me = useAppSelector(selectCurrentUser);
  const { lastView } = useAppSelector(selectQueue);
  const { data: ticket, isLoading, isError, error, refetch } = useGetTicketQuery(number);
  const [update, { isLoading: solving }] = useUpdateTicketMutation();
  const back = (
    <Link
      href={`/agent/tickets?view=${lastView}`}
      className="inline-flex items-center gap-1.5 self-start rounded-md text-sm text-muted-foreground hover:text-foreground"
    >
      <ArrowLeftIcon className="size-4" />
      {QUEUE_VIEWS[lastView].label}
    </Link>
  );

  if (isError) {
    const missing = errorStatus(error) === 404;
    return (
      <div className="flex flex-col gap-5">
        {back}
        <EmptyState
          icon={SearchXIcon}
          title={missing ? `${ticketRef(number)} doesn't exist` : "We couldn't load this ticket"}
          action={
            !missing && (
              <Button variant="outline" onClick={() => refetch()}>
                Try again
              </Button>
            )
          }
        >
          {missing ? "Check the ticket number, or go back to the queue." : errorMessage(error)}
        </EmptyState>
      </div>
    );
  }

  if (isLoading || !ticket) {
    return (
      <div className="flex flex-col gap-4">
        {back}
        <Skeleton className="h-8 w-2/3" />
        <Skeleton className="h-32 w-full max-w-3xl" />
        <Skeleton className="h-24 w-full max-w-3xl" />
      </div>
    );
  }

  const solve = async () => {
    try {
      await update({ number, status: "solved" }).unwrap();
      toast.success(`${ticketRef(number)} marked solved`);
    } catch (err) {
      toast.error(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  };

  return (
    <div className="flex flex-col gap-5">
      {back}
      <header className="flex flex-wrap items-start gap-x-4 gap-y-3">
        <div className="min-w-0 flex-1 basis-72">
          <p className="font-mono text-xs text-faint">{ticketRef(ticket.number)}</p>
          <h1 className="text-xl font-semibold tracking-tight text-balance">{ticket.subject}</h1>
        </div>
        <div className="flex items-center gap-2">
          <StatusBadge status={ticket.status} />
          {isActive(ticket.status) && (
            <Button variant="outline" size="sm" onClick={solve} disabled={solving}>
              <CheckCircle2Icon />
              Mark solved
            </Button>
          )}
        </div>
      </header>

      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_18rem]">
        <div className="flex min-w-0 flex-col gap-5">
          <AgentThread ticket={ticket} meId={me?.id} />
          <AgentComposer key={ticket.number} ticket={ticket} />
        </div>
        <div className="lg:border-l lg:pl-6">
          <TicketProperties ticket={ticket} meId={me?.id} />
        </div>
      </div>
    </div>
  );
}
