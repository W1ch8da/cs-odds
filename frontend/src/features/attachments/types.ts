export type Attachment = {
  id: string;
  filename: string;
  contentType: string;
  size: number;
  createdAt: string;
};

export type UploadSlot = {
  attachment: Attachment;
  /** Presigned URL for one `PUT` of the file bytes, straight to storage. */
  uploadUrl: string;
  /** Headers the `PUT` must send exactly; they're part of the signature. */
  uploadHeaders: Record<string, string>;
  expiresAt: string;
};

/** Mirrors the backend limits in domain/attachment.rs. */
export const MAX_FILE_BYTES = 25 * 1024 * 1024;
export const MAX_FILES_PER_MESSAGE = 10;
