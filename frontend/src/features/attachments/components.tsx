"use client";

import { CircleAlertIcon, FileIcon, FileTextIcon, ImageIcon, Loader2Icon, PaperclipIcon, XIcon } from "lucide-react";
import { useId, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { formatBytes } from "@/lib/format";
import { cn } from "@/lib/utils";

import { downloadUrl } from "./attachmentsApi";
import type { Attachment } from "./types";
import type { Upload } from "./useUploads";

function iconFor(contentType: string) {
  if (contentType.startsWith("image/")) return ImageIcon;
  if (contentType.startsWith("text/") || contentType === "application/pdf") return FileTextIcon;
  return FileIcon;
}

/** Downloadable files under a message. */
export function AttachmentList({ attachments, className }: { attachments: Attachment[]; className?: string }) {
  if (attachments.length === 0) return null;
  return (
    <ul className={cn("flex flex-wrap gap-2", className)} aria-label="Attachments">
      {attachments.map((a) => {
        const Icon = iconFor(a.contentType);
        return (
          <li key={a.id}>
            <a
              href={downloadUrl(a.id)}
              title={`Download ${a.filename}`}
              className="inline-flex max-w-72 items-center gap-2 rounded-lg border bg-muted/60 px-2.5 py-1.5 text-[13px] text-muted-foreground transition-colors hover:border-input hover:text-foreground"
            >
              <Icon className="size-4 shrink-0" />
              <span className="truncate font-medium text-foreground">{a.filename}</span>
              <span className="shrink-0 tabular-nums">{formatBytes(a.size)}</span>
            </a>
          </li>
        );
      })}
    </ul>
  );
}

/** Files being attached to a message that hasn't been sent yet. */
export function UploadList({ uploads, onRemove }: { uploads: Upload[]; onRemove: (key: string) => void }) {
  if (uploads.length === 0) return null;
  return (
    <ul className="flex flex-col gap-1.5" aria-label="Files to attach">
      {uploads.map((u) => (
        <li
          key={u.key}
          className={cn(
            "relative flex items-center gap-2 overflow-hidden rounded-lg border bg-card px-2.5 py-1.5 text-[13px]",
            u.status === "error" && "border-destructive/40 bg-destructive-soft",
          )}
        >
          {u.status === "uploading" && (
            <span aria-hidden className="absolute inset-y-0 left-0 bg-accent transition-[width]" style={{ width: `${Math.round(u.progress * 100)}%` }} />
          )}
          <span className="relative flex min-w-0 flex-1 items-center gap-2">
            {u.status === "uploading" ? (
              <Loader2Icon className="size-4 shrink-0 animate-spin text-muted-foreground" />
            ) : u.status === "error" ? (
              <CircleAlertIcon className="size-4 shrink-0 text-destructive" />
            ) : (
              <PaperclipIcon className="size-4 shrink-0 text-muted-foreground" />
            )}
            <span className="truncate font-medium">{u.name}</span>
            <span className="shrink-0 text-faint tabular-nums">
              {u.status === "uploading" ? `${Math.round(u.progress * 100)}%` : formatBytes(u.size)}
            </span>
            {u.error && <span className="truncate text-destructive">{u.error}</span>}
          </span>
          <button
            type="button"
            onClick={() => onRemove(u.key)}
            aria-label={`Remove ${u.name}`}
            className="relative grid size-6 shrink-0 place-items-center rounded text-faint hover:bg-muted hover:text-foreground"
          >
            <XIcon className="size-3.5" />
          </button>
        </li>
      ))}
    </ul>
  );
}

/** "Attach" button that opens the file picker. */
export function AttachButton({ onFiles, disabled = false }: { onFiles: (files: FileList) => void; disabled?: boolean }) {
  const inputRef = useRef<HTMLInputElement>(null);
  return (
    <>
      <Button type="button" variant="ghost" size="sm" disabled={disabled} onClick={() => inputRef.current?.click()}>
        <PaperclipIcon />
        Attach
      </Button>
      <input
        ref={inputRef}
        type="file"
        multiple
        hidden
        onChange={(e) => {
          if (e.target.files?.length) onFiles(e.target.files);
          e.target.value = "";
        }}
      />
    </>
  );
}

/** Large drop target with a picker, for forms. */
export function DropZone({ onFiles }: { onFiles: (files: FileList) => void }) {
  const id = useId();
  const [over, setOver] = useState(false);
  return (
    <label
      htmlFor={id}
      onDragOver={(e) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setOver(false);
        if (e.dataTransfer.files.length) onFiles(e.dataTransfer.files);
      }}
      className={cn(
        "flex cursor-pointer flex-col items-center gap-1 rounded-lg border-[1.5px] border-dashed border-input px-4 py-5 text-center text-sm text-muted-foreground transition-colors hover:border-primary hover:bg-accent/50",
        over && "border-primary bg-accent text-foreground",
      )}
    >
      <PaperclipIcon className="size-4" />
      <span>
        <span className="font-medium text-accent-foreground">Choose files</span> or drag them here
      </span>
      <span className="text-xs text-faint">Up to 10 files, 25 MB each</span>
      <input
        id={id}
        type="file"
        multiple
        className="sr-only"
        onChange={(e) => {
          if (e.target.files?.length) onFiles(e.target.files);
          e.target.value = "";
        }}
      />
    </label>
  );
}
