"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { ArrowLeftIcon, CircleAlertIcon, Loader2Icon } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { toast } from "sonner";
import { z } from "zod";

import { FormField, fieldA11y } from "@/components/forms/FormField";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { selectCurrentUser } from "@/features/auth/authSlice";
import { useListTeamQuery } from "@/features/users/usersApi";
import { errorMessage } from "@/lib/api-error";
import { useAppSelector } from "@/lib/store/hooks";

import { PRIORITIES, PRIORITY_LABEL, ticketRef } from "../labels";
import { useCreateTicketMutation } from "../ticketsApi";

const UNASSIGNED = "unassigned";

const schema = z.object({
  requesterEmail: z.string().trim().min(1, "Enter the customer's email.").pipe(z.email("Enter a valid email address.")),
  requesterName: z.string().trim().max(100, "Keep the name under 100 characters."),
  subject: z.string().trim().min(1, "Add a subject.").max(200, "Keep the subject under 200 characters."),
  description: z.string().trim().min(1, "Describe what the customer needs."),
  priority: z.enum(["low", "normal", "high", "urgent"]),
  assignee: z.string(),
});

type Values = z.infer<typeof schema>;

/** Log a request for a customer, e.g. from a phone call. */
export function NewTicketForm() {
  const router = useRouter();
  const me = useAppSelector(selectCurrentUser);
  const team = useListTeamQuery();
  const [create, { isLoading }] = useCreateTicketMutation();
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    control,
    handleSubmit,
    formState: { errors },
  } = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: { requesterEmail: "", requesterName: "", subject: "", description: "", priority: "normal", assignee: me?.id ?? UNASSIGNED },
  });

  const assigneeItems = [
    { value: UNASSIGNED, label: "Unassigned" },
    ...(team.data ?? []).map((u) => ({ value: u.id, label: u.id === me?.id ? `${u.name} (you)` : u.name })),
  ];
  const priorityItems = PRIORITIES.map((p) => ({ value: p, label: PRIORITY_LABEL[p] }));

  const onSubmit = handleSubmit(async ({ assignee, requesterName, ...values }) => {
    setFormError(null);
    try {
      const ticket = await create({
        ...values,
        requesterName: requesterName || undefined,
        assigneeId: assignee === UNASSIGNED ? undefined : assignee,
      }).unwrap();
      toast.success(`${ticketRef(ticket.number)} created for ${ticket.requester.name}`);
      router.push(`/agent/tickets/${ticket.number}`);
    } catch (err) {
      setFormError(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  });

  return (
    <div className="flex max-w-2xl flex-col gap-5">
      <Link href="/agent/tickets" className="inline-flex items-center gap-1.5 self-start text-sm text-muted-foreground hover:text-foreground">
        <ArrowLeftIcon className="size-4" />
        Tickets
      </Link>
      <div>
        <h1 className="text-xl font-semibold tracking-tight">New ticket</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Log a request for a customer, for example after a phone call. New customers are added automatically.
        </p>
      </div>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-5 rounded-xl border bg-card p-5 sm:p-6">
        {formError && (
          <Alert variant="destructive">
            <CircleAlertIcon />
            <AlertDescription>{formError}</AlertDescription>
          </Alert>
        )}
        <div className="grid gap-5 sm:grid-cols-2">
          <FormField id="requester-email" label="Customer email" error={errors.requesterEmail?.message}>
            <Input {...fieldA11y("requester-email", errors.requesterEmail?.message)} type="email" autoFocus {...register("requesterEmail")} />
          </FormField>
          <FormField id="requester-name" label="Customer name" hint="Needed only for new customers." error={errors.requesterName?.message}>
            <Input {...fieldA11y("requester-name", errors.requesterName?.message, "Needed only for new customers.")} {...register("requesterName")} />
          </FormField>
        </div>
        <FormField id="subject" label="Subject" error={errors.subject?.message}>
          <Input {...fieldA11y("subject", errors.subject?.message)} {...register("subject")} />
        </FormField>
        <FormField id="description" label="Description" error={errors.description?.message}>
          <textarea
            {...fieldA11y("description", errors.description?.message)}
            {...register("description")}
            rows={6}
            className="w-full resize-y rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 aria-invalid:border-destructive dark:bg-input/30"
          />
        </FormField>
        <div className="grid gap-5 sm:grid-cols-2">
          <FormField id="priority" label="Priority">
            <Controller
              control={control}
              name="priority"
              render={({ field }) => (
                <Select items={priorityItems} value={field.value} onValueChange={(v) => v && field.onChange(v)}>
                  <SelectTrigger id="priority" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {priorityItems.map((i) => (
                      <SelectItem key={i.value} value={i.value}>
                        {i.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            />
          </FormField>
          <FormField id="assignee" label="Assignee">
            <Controller
              control={control}
              name="assignee"
              render={({ field }) => (
                <Select items={assigneeItems} value={field.value} onValueChange={(v) => v && field.onChange(v)}>
                  <SelectTrigger id="assignee" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {assigneeItems.map((i) => (
                      <SelectItem key={i.value} value={i.value}>
                        {i.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            />
          </FormField>
        </div>
        <div className="flex gap-2">
          <Button type="submit" disabled={isLoading}>
            {isLoading && <Loader2Icon className="animate-spin" />}
            Create ticket
          </Button>
          <Button variant="ghost" nativeButton={false} render={<Link href="/agent/tickets" />}>
            Cancel
          </Button>
        </div>
      </form>
    </div>
  );
}
