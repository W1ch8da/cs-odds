import type { SerializedError } from "@reduxjs/toolkit";
import type { FetchBaseQueryError } from "@reduxjs/toolkit/query";

import type { ApiErrorBody } from "@/lib/store/api";

type AnyError = FetchBaseQueryError | SerializedError | undefined;

function body(error: AnyError): ApiErrorBody["error"] | undefined {
  if (error && "status" in error && typeof error.data === "object" && error.data && "error" in error.data) {
    return (error.data as ApiErrorBody).error;
  }
  return undefined;
}

/** A message that can be shown to the user as-is. */
export function errorMessage(error: AnyError): string {
  const apiError = body(error);
  if (apiError) return apiError.message;
  if (error && "status" in error && (error.status === "FETCH_ERROR" || error.status === "TIMEOUT_ERROR")) {
    return "We can't reach the server. Check your connection and try again.";
  }
  return "Something went wrong. Try again in a moment.";
}

export function errorStatus(error: AnyError): number | string | undefined {
  return error && "status" in error ? error.status : undefined;
}
