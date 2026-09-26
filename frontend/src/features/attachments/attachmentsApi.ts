import { baseApi } from "@/lib/store/api";

import type { UploadSlot } from "./types";

export type StartUploadRequest = { filename: string; contentType: string; size: number };

export const attachmentsApi = baseApi.injectEndpoints({
  endpoints: (build) => ({
    startUpload: build.mutation<UploadSlot, StartUploadRequest>({
      query: (body) => ({ url: "/attachments", method: "POST", body }),
    }),
  }),
});

export const { useStartUploadMutation } = attachmentsApi;

/** Same-origin link that checks access, then redirects to the file. */
export function downloadUrl(id: string): string {
  return `/api/attachments/${id}/download`;
}
