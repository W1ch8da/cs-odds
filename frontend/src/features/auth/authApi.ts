import { baseApi } from "@/lib/store/api";

import type { Session, User } from "./types";

export type LoginRequest = { email: string; password: string };
export type RegisterRequest = { name: string; email: string; password: string };

/**
 * After signing in or registering, store the user as the /auth/me result so
 * guarded pages render without another request. Failures are shown by the
 * form that called the mutation, so they're ignored here.
 */
async function seedCurrentUser(
  _arg: unknown,
  { dispatch, queryFulfilled }: { dispatch: (action: unknown) => unknown; queryFulfilled: Promise<{ data: Session }> },
) {
  try {
    const { data } = await queryFulfilled;
    dispatch(authApi.util.upsertQueryData("me", undefined, data.user));
  } catch {
    // Handled by the caller.
  }
}

export const authApi = baseApi.injectEndpoints({
  endpoints: (build) => ({
    me: build.query<User, void>({
      query: () => "/auth/me",
      providesTags: ["Me"],
    }),
    login: build.mutation<Session, LoginRequest>({
      query: (body) => ({ url: "/auth/login", method: "POST", body }),
      onQueryStarted: seedCurrentUser,
    }),
    register: build.mutation<Session, RegisterRequest>({
      query: (body) => ({ url: "/auth/register", method: "POST", body }),
      onQueryStarted: seedCurrentUser,
    }),
    logout: build.mutation<void, void>({
      query: () => ({ url: "/auth/logout", method: "POST" }),
    }),
  }),
});

export const { useMeQuery, useLoginMutation, useRegisterMutation, useLogoutMutation } = authApi;
