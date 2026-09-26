"use client";

import { toast } from "sonner";

import { PersonAvatar } from "@/components/PersonAvatar";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useListTeamQuery } from "@/features/users/usersApi";
import { errorMessage } from "@/lib/api-error";
import { formatDateTime } from "@/lib/format";

import { CHANNEL_LABEL, PRIORITIES, PRIORITY_LABEL, STATUSES, STATUS_LABEL } from "../labels";
import { useUpdateTicketMutation } from "../ticketsApi";
import type { Priority, TicketDetail, TicketStatus, UpdateTicketRequest } from "../types";

const UNASSIGNED = "unassigned";

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h2 className="mb-2.5 text-[11px] font-medium tracking-wider text-faint uppercase">{children}</h2>;
}

type FieldProps<T extends string> = {
  id: string;
  label: string;
  value: T;
  items: { value: T; label: string }[];
  disabled: boolean;
  onChange: (value: T) => void;
};

function Field<T extends string>({ id, label, value, items, disabled, onChange }: FieldProps<T>) {
  return (
    <div className="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-2">
      <label htmlFor={id} className="text-sm text-muted-foreground">
        {label}
      </label>
      <Select items={items} value={value} disabled={disabled} onValueChange={(v) => v != null && v !== value && onChange(v as T)}>
        <SelectTrigger id={id} className="w-full bg-card">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {items.map((i) => (
            <SelectItem key={i.value} value={i.value}>
              {i.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

/** Status, priority, assignee and requester details. Changes save immediately. */
export function TicketProperties({ ticket, meId }: { ticket: TicketDetail; meId?: string }) {
  const [update, { isLoading }] = useUpdateTicketMutation();
  const team = useListTeamQuery();
  const closed = ticket.status === "closed";

  const save = async (patch: Omit<UpdateTicketRequest, "number">, done: string) => {
    try {
      await update({ number: ticket.number, ...patch }).unwrap();
      toast.success(done);
    } catch (err) {
      toast.error(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  };

  const assigneeItems = [
    { value: UNASSIGNED, label: "Unassigned" },
    ...(team.data ?? []).map((u) => ({ value: u.id, label: u.id === meId ? `${u.name} (you)` : u.name })),
  ];
  // Keep the current assignee selectable even before the team list loads.
  if (ticket.assignee && !assigneeItems.some((i) => i.value === ticket.assignee!.id)) {
    assigneeItems.push({ value: ticket.assignee.id, label: ticket.assignee.name });
  }

  return (
    <aside aria-label="Ticket details" className="flex flex-col gap-7">
      <section>
        <SectionTitle>Properties</SectionTitle>
        <div className="flex flex-col gap-2">
          <Field<TicketStatus>
            id="prop-status"
            label="Status"
            value={ticket.status}
            items={STATUSES.map((s) => ({ value: s, label: STATUS_LABEL[s] }))}
            disabled={closed || isLoading}
            onChange={(status) => save({ status }, `Status set to ${STATUS_LABEL[status]}`)}
          />
          <Field<Priority>
            id="prop-priority"
            label="Priority"
            value={ticket.priority}
            items={PRIORITIES.map((p) => ({ value: p, label: PRIORITY_LABEL[p] }))}
            disabled={closed || isLoading}
            onChange={(priority) => save({ priority }, `Priority set to ${PRIORITY_LABEL[priority]}`)}
          />
          <Field<string>
            id="prop-assignee"
            label="Assignee"
            value={ticket.assignee?.id ?? UNASSIGNED}
            items={assigneeItems}
            disabled={closed || isLoading}
            onChange={(id) =>
              save(
                { assigneeId: id === UNASSIGNED ? null : id },
                id === UNASSIGNED ? "Ticket unassigned" : `Assigned to ${id === meId ? "you" : assigneeItems.find((i) => i.value === id)?.label}`,
              )
            }
          />
          {!closed && meId && ticket.assignee?.id !== meId && (
            <button
              type="button"
              disabled={isLoading}
              onClick={() => save({ assigneeId: meId }, "Assigned to you")}
              className="ml-[5.5rem] self-start text-sm font-medium text-accent-foreground hover:underline disabled:opacity-50"
            >
              Assign to me
            </button>
          )}
        </div>
      </section>

      <section>
        <SectionTitle>Requester</SectionTitle>
        <div className="flex items-center gap-3">
          <PersonAvatar name={ticket.requester.name} className="size-9" />
          <div className="min-w-0 leading-tight">
            <p className="truncate font-medium">{ticket.requester.name}</p>
            <p className="truncate text-sm text-muted-foreground">{ticket.requester.email}</p>
          </div>
        </div>
        <dl className="mt-3 grid grid-cols-[5rem_minmax(0,1fr)] gap-x-2 gap-y-1.5 text-sm">
          <dt className="text-muted-foreground">Tickets</dt>
          <dd>{ticket.requesterTicketCount} in total</dd>
        </dl>
      </section>

      <section>
        <SectionTitle>Details</SectionTitle>
        <dl className="grid grid-cols-[5rem_minmax(0,1fr)] gap-x-2 gap-y-1.5 text-sm">
          <dt className="text-muted-foreground">Channel</dt>
          <dd>{CHANNEL_LABEL[ticket.channel]}</dd>
          <dt className="text-muted-foreground">Created</dt>
          <dd>{formatDateTime(ticket.createdAt)}</dd>
          <dt className="text-muted-foreground">Updated</dt>
          <dd>{formatDateTime(ticket.updatedAt)}</dd>
          {ticket.resolvedAt && (
            <>
              <dt className="text-muted-foreground">Solved</dt>
              <dd>{formatDateTime(ticket.resolvedAt)}</dd>
            </>
          )}
        </dl>
      </section>
    </aside>
  );
}
