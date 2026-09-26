"use client";

import { useCallback, useRef, useState } from "react";

import { errorMessage } from "@/lib/api-error";
import { formatBytes } from "@/lib/format";

import { useStartUploadMutation } from "./attachmentsApi";
import { MAX_FILE_BYTES, MAX_FILES_PER_MESSAGE } from "./types";

export type Upload = {
  key: string;
  name: string;
  size: number;
  status: "uploading" | "done" | "error";
  /** 0–1 */
  progress: number;
  attachmentId?: string;
  error?: string;
};

/** PUT the file to storage, reporting progress (fetch can't report upload progress). */
function putFile(url: string, headers: Record<string, string>, file: File, onProgress: (p: number) => void) {
  return new Promise<void>((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("PUT", url);
    for (const [name, value] of Object.entries(headers)) xhr.setRequestHeader(name, value);
    xhr.upload.onprogress = (e) => e.lengthComputable && onProgress(e.loaded / e.total);
    xhr.onload = () =>
      xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new Error("The upload was rejected. Try again."));
    xhr.onerror = () => reject(new Error("We couldn't reach file storage. Check your connection and try again."));
    xhr.send(file);
  });
}

/**
 * Uploads files as soon as they're picked, so sending a message only has to
 * pass along the finished attachment ids.
 */
export function useUploads() {
  const [uploads, setUploads] = useState<Upload[]>([]);
  const [startUpload] = useStartUploadMutation();
  const counter = useRef(0);

  const patch = useCallback((key: string, changes: Partial<Upload>) => {
    setUploads((list) => list.map((u) => (u.key === key ? { ...u, ...changes } : u)));
  }, []);

  const add = useCallback(
    (files: FileList | File[]) => {
      const picked = Array.from(files);
      setUploads((current) => {
        const room = MAX_FILES_PER_MESSAGE - current.filter((u) => u.status !== "error").length;
        const next = [...current];
        picked.forEach((file, i) => {
          const key = `upload-${counter.current++}`;
          const base = { key, name: file.name, size: file.size, progress: 0 };
          if (i >= room) {
            next.push({ ...base, status: "error", error: `You can attach up to ${MAX_FILES_PER_MESSAGE} files.` });
          } else if (file.size > MAX_FILE_BYTES) {
            next.push({ ...base, status: "error", error: `Over 25 MB (${formatBytes(file.size)}). Try a smaller file or a share link.` });
          } else if (file.size === 0) {
            next.push({ ...base, status: "error", error: "This file is empty." });
          } else {
            next.push({ ...base, status: "uploading" });
            void (async () => {
              try {
                const slot = await startUpload({ filename: file.name, contentType: file.type, size: file.size }).unwrap();
                await putFile(slot.uploadUrl, slot.uploadHeaders, file, (progress) => patch(key, { progress }));
                patch(key, { status: "done", progress: 1, attachmentId: slot.attachment.id });
              } catch (err) {
                const message = err instanceof Error ? err.message : errorMessage(err as Parameters<typeof errorMessage>[0]);
                patch(key, { status: "error", error: message });
              }
            })();
          }
        });
        return next;
      });
    },
    [patch, startUpload],
  );

  const remove = useCallback((key: string) => setUploads((list) => list.filter((u) => u.key !== key)), []);
  const reset = useCallback(() => setUploads([]), []);

  return {
    uploads,
    add,
    remove,
    reset,
    attachmentIds: uploads.flatMap((u) => (u.status === "done" && u.attachmentId ? [u.attachmentId] : [])),
    busy: uploads.some((u) => u.status === "uploading"),
    failed: uploads.some((u) => u.status === "error"),
  };
}
