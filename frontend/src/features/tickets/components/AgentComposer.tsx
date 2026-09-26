"use client";

import { Loader2Icon, LockIcon, MailIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { AttachButton, UploadList } from "@/features/attachments/components";
import { useUploads } from "@/features/attachments/useUploads";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { errorMessage } from "@/lib/api-error";
import { firstName } from "@/lib/format";
import { cn } from "@/lib/utils";

import { useAddCommentMutation } from "../ticketsApi";
import type { TicketDetail, TicketStatus } from "../types";

type Mode = "reply" | "note";
type After = TicketStatus | "keep";

const AFTER_ITEMS: { value: After; label: string }[] = [
  { value: "pending", label: "Pending" },
  { value: "keep", label: "Keep status" },
  { value: "solved", label: "Solved" },
];

/** Reply to the customer or leave an internal note. R and N jump here. */
export function AgentComposer({ ticket }: { ticket: TicketDetail }) {
  const [mode, setMode] = useState<Mode>("reply");
  const [body, setBody] = useState("");
  const [after, setAfter] = useState<After>("pending");
  const [error, setError] = useState<string | null>(null);
  const [addComment, { isLoading }] = useAddCommentMutation();
  const files = useUploads();
  const textRef = useRef<HTMLTextAreaElement>(null);
  const customer = firstName(ticket.requester.name);
  const note = mode === "note";

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (/INPUT|TEXTAREA|SELECT/.test(target.tagName) || target.isContentEditable || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "r" || e.key === "n") {
        e.preventDefault();
        setMode(e.key === "n" ? "note" : "reply");
        textRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

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
      setError(note ? "Write the note first." : "Write your reply first.");
      textRef.current?.focus();
      return;
    }
    setError(null);
    try {
      await addComment({
        number: ticket.number,
        body,
        internal: note,
        status: !note && after !== "keep" ? after : undefined,
        attachmentIds: files.attachmentIds,
      }).unwrap();
      setBody("");
      files.reset();
      toast.success(note ? "Note added. Only your team can see it." : `Reply sent to ${ticket.requester.name}`);
    } catch (err) {
      setError(errorMessage(err as Parameters<typeof errorMessage>[0]));
    }
  };

  if (ticket.status === "closed") {
    return (
      <p className="rounded-xl border bg-muted/50 px-4 py-3 text-sm text-muted-foreground">
        This ticket is closed, so it can&apos;t take new replies. Create a new ticket if the customer needs more help.
      </p>
    );
  }

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        void send();
      }}
      className={cn("flex flex-col gap-2 rounded-xl border bg-card p-3 transition-colors", note && "border-note-border bg-note")}
    >
      <div role="tablist" aria-label="Message type" className="flex gap-1">
        {(
          [
            ["reply", MailIcon, `Reply to ${customer}`],
            ["note", LockIcon, "Internal note"],
          ] as const
        ).map(([value, Icon, label]) => (
          <button
            key={value}
            type="button"
            role="tab"
            aria-selected={mode === value}
            onClick={() => {
              setMode(value);
              textRef.current?.focus();
            }}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-md px-2.5 py-1 text-[13px] font-medium text-muted-foreground",
              mode === value && (note ? "bg-note-border text-foreground" : "bg-muted text-foreground"),
            )}
          >
            <Icon className="size-3.5" />
            {label}
          </button>
        ))}
      </div>
      <textarea
        ref={textRef}
        id="composer"
        value={body}
        onChange={(e) => setBody(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            void send();
          }
        }}
        aria-label={note ? "Internal note" : `Reply to ${customer}`}
        aria-invalid={error ? true : undefined}
        placeholder={note ? "Only your team can see this note" : `Write a reply. ${customer} will see it in the portal and by email.`}
        className="min-h-24 w-full resize-y rounded-lg border bg-card px-3 py-2 text-sm outline-none placeholder:text-faint focus:border-ring"
      />
      <UploadList uploads={files.uploads} onRemove={files.remove} />
      {error && (
        <p role="alert" className="text-xs text-destructive">
          {error}
        </p>
      )}
      <div className="flex flex-wrap items-center gap-2">
        <AttachButton onFiles={files.add} />
        <span className="text-xs text-faint">
          <kbd className="rounded border px-1 font-mono">⌘</kbd> <kbd className="rounded border px-1 font-mono">Enter</kbd> to send
        </span>
        <div className="ml-auto flex items-center gap-2">
          {!note && (
            <>
              <label htmlFor="after-status" className="text-xs text-faint">
                then set
              </label>
              <Select items={AFTER_ITEMS} value={after} onValueChange={(v) => v && setAfter(v as After)}>
                <SelectTrigger id="after-status" size="sm" className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {AFTER_ITEMS.map((i) => (
                    <SelectItem key={i.value} value={i.value}>
                      {i.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </>
          )}
          <Button type="submit" size="sm" disabled={isLoading || files.busy}>
            {isLoading && <Loader2Icon className="animate-spin" />}
            {note ? "Add note" : "Send reply"}
          </Button>
        </div>
      </div>
    </form>
  );
}
