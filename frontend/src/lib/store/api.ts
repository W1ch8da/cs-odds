import {
  createApi,
  fetchBaseQuery,
  type BaseQueryFn,
  type FetchArgs,
  type FetchBaseQueryError,
} from "@reduxjs/toolkit/query/react";

import { sessionExpired } from "./authEvents";

/** Error body returned by every backend endpoint. */
export type ApiErrorBody = { error: { code: string; message: string } };

const rawBaseQuery = fetchBaseQuery({
  // Same-origin rewrite to the Rust API (next.config.ts), so auth cookies just work.
  baseUrl: "/api",
  credentials: "include",
});

// A 401 from these means "bad credentials", not "access token expired".
const NO_REFRESH = new Set(["/auth/login", "/auth/register", "/auth/refresh", "/auth/logout"]);

// Shared so that parallel 401s trigger a single refresh request.
let refreshing: Promise<boolean> | null = null;

/**
 * On 401, exchanges the refresh cookie for a new session once, then retries
 * the original request. If refreshing fails, the session is over.
 */
const baseQueryWithReauth: BaseQueryFn<string | FetchArgs, unknown, FetchBaseQueryError> = async (
  args,
  api,
  extraOptions,
) => {
  let result = await rawBaseQuery(args, api, extraOptions);
  const url = typeof args === "string" ? args : args.url;

  if (result.error?.status === 401 && !NO_REFRESH.has(url)) {
    refreshing ??= Promise.resolve(rawBaseQuery({ url: "/auth/refresh", method: "POST" }, api, extraOptions))
      .then((res) => !res.error)
      .finally(() => {
        refreshing = null;
      });

    if (await refreshing) {
      result = await rawBaseQuery(args, api, extraOptions);
    } else {
      api.dispatch(sessionExpired());
    }
  }
  return result;
};

/**
 * Single RTK Query API for the Rust backend. Features add their endpoints
 * with `baseApi.injectEndpoints` (see src/features/*).
 */
export const baseApi = createApi({
  reducerPath: "api",
  baseQuery: baseQueryWithReauth,
  tagTypes: ["Me", "User", "Ticket", "Comment", "Dashboard"],
  endpoints: () => ({}),
});
