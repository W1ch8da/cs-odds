"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { ArrowLeftIcon, CircleAlertIcon, Loader2Icon } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useForm, useWatch } from "react-hook-form";
import { z } from "zod";

import { FormField, fieldA11y } from "@/components/forms/FormField";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DropZone, UploadList } from "@/features/attachments/components";
import { useUploads } from "@/features/attachments/useUploads";
import { IMPACT_OPTIONS } from "@/features/tickets/labels";
import { useCreateTicketMutation } from "@/features/tickets/ticketsApi";
import { errorMessage } from "@/lib/api-error";
import { cn } from "@/lib/utils";

const schema = z.object({
  subject: z
    .string()
    .trim()
    .min(1, "Add a subject so we can route your request.")
    .min(5, "Add a few more words so we know what this is about.")
    .max(200, "Keep the subject under 200 characters."),
  priority: z.enum(["low", "normal", "high", "urgent"]),
  description: z
    .string()
    .trim()
    .min(15, "Describe the problem in a sentence or two. What happened, and what did you expect?"),
});

type Values = z.infer<typeof schema>;
const DETAILS_HINT = "What happened, what you expected, and any error messages you saw.";

export function NewRequestForm() {
  const router = useRouter();
  const [create, { isLoading }] = useCreateTicketMutation();
  const [formError, setFormError] = useState<string | null>(null);
  const files = useUploads();
  const {
    register,
    handleSubmit,
    control,
    formState: { errors },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { subject: "", priority: "normal", description: "" } });
  const priority = useWatch({ control, name: "priority" });

  const onSubmit = handleSubmit(async (values) => {
    setFormError(null);
    if (files.busy) {
      setFormError("Wait for your files to finish uploading.");
      return;
    }
    if (files.failed) {
      setFormError("Remove the files that couldn't be uploaded, then send.");
      return;
    }
    try {
      const ticket = await create({ ...values, attachmentIds: files.attachmentIds }).unwrap();
      router.push(`/portal/requests/${ticket.number}?created=1`);
    } catch (err) {
      setFormError(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  });

  return (
    <>
      <div className="flex flex-col gap-2">
        <Link href="/portal" className="inline-flex items-center gap-1.5 self-start text-sm text-muted-foreground hover:text-foreground">
          <ArrowLeftIcon className="size-4" />
          My requests
        </Link>
        <h1 className="text-2xl font-semibold tracking-tight">New request</h1>
        <p className="max-w-prose text-muted-foreground">Tell us what&apos;s going on. The more detail you share, the faster we can help.</p>
      </div>

      <div className="grid items-start gap-7 lg:grid-cols-[minmax(0,1fr)_16rem]">
        <form onSubmit={onSubmit} noValidate className="flex flex-col gap-6 rounded-xl border bg-card p-5 sm:p-6">
          {formError && (
            <Alert variant="destructive">
              <CircleAlertIcon />
              <AlertDescription>{formError}</AlertDescription>
            </Alert>
          )}
          <FormField id="subject" label="Subject" error={errors.subject?.message}>
            <Input
              {...fieldA11y("subject", errors.subject?.message)}
              autoFocus
              autoComplete="off"
              placeholder="For example: Invoices show the wrong address"
              {...register("subject")}
            />
          </FormField>

          <fieldset className="flex flex-col gap-2">
            <legend className="mb-2 text-sm font-medium">How is this affecting you?</legend>
            <div className="grid gap-2 sm:grid-cols-2">
              {IMPACT_OPTIONS.map((option) => (
                <label
                  key={option.priority}
                  className={cn(
                    "flex cursor-pointer items-start gap-3 rounded-lg border px-3 py-2.5 transition-colors hover:border-input",
                    priority === option.priority && "border-primary bg-accent hover:border-primary",
                  )}
                >
                  <input type="radio" value={option.priority} {...register("priority")} className="mt-1 accent-primary" />
                  <span className="flex flex-col">
                    <span className="text-sm font-medium">{option.title}</span>
                    <span className="text-[13px] text-muted-foreground">{option.description}</span>
                  </span>
                </label>
              ))}
            </div>
          </fieldset>

          <FormField id="description" label="Details" hint={DETAILS_HINT} error={errors.description?.message}>
            <textarea
              {...fieldA11y("description", errors.description?.message, DETAILS_HINT)}
              {...register("description")}
              rows={7}
              placeholder="Steps to reproduce, account or invoice numbers, when it started…"
              className="w-full resize-y rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm outline-none placeholder:text-faint focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 aria-invalid:border-destructive dark:bg-input/30"
            />
          </FormField>

          <div className="flex flex-col gap-2">
            <span className="text-sm font-medium">
              Attachments <span className="font-normal text-faint">(optional)</span>
            </span>
            <DropZone onFiles={files.add} />
            <UploadList uploads={files.uploads} onRemove={files.remove} />
          </div>

          <div className="flex flex-wrap gap-2">
            <Button type="submit" size="lg" disabled={isLoading || files.busy}>
              {isLoading && <Loader2Icon className="animate-spin" />}
              {isLoading ? "Sending…" : "Send request"}
            </Button>
            <Button variant="ghost" size="lg" nativeButton={false} render={<Link href="/portal" />}>
              Cancel
            </Button>
          </div>
        </form>

        <aside className="rounded-xl border bg-card p-5 text-sm text-muted-foreground">
          <h2 className="mb-3 font-semibold text-foreground">What happens next</h2>
          <ol className="flex flex-col gap-3">
            {[
              "You get a request number straight away.",
              "Our team replies here, and we email you a copy.",
              "Reply to keep the conversation going, or mark it solved when you're done.",
            ].map((step, i) => (
              <li key={step} className="grid grid-cols-[1.375rem_minmax(0,1fr)] gap-2.5">
                <span className="grid size-5.5 place-items-center rounded-full border bg-muted text-xs tabular-nums">{i + 1}</span>
                <span>{step}</span>
              </li>
            ))}
          </ol>
        </aside>
      </div>
    </>
  );
}
