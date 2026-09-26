import { createSlice, isAnyOf } from "@reduxjs/toolkit";

import { sessionExpired } from "@/lib/store/authEvents";

import { authApi } from "./authApi";
import type { User } from "./types";

type AuthState = {
  user: User | null;
  /** "unknown" until the first /auth/me answer arrives. */
  status: "unknown" | "authenticated" | "anonymous";
};

const initialState: AuthState = { user: null, status: "unknown" };

/**
 * Who is signed in. Filled from the auth endpoints; the tokens themselves
 * live in httpOnly cookies and never reach JavaScript.
 */
export const authSlice = createSlice({
  name: "auth",
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    builder
      .addMatcher(
        isAnyOf(authApi.endpoints.login.matchFulfilled, authApi.endpoints.register.matchFulfilled),
        (state, { payload }) => {
          state.user = payload.user;
          state.status = "authenticated";
        },
      )
      .addMatcher(authApi.endpoints.me.matchFulfilled, (state, { payload }) => {
        state.user = payload;
        state.status = "authenticated";
      })
      .addMatcher(authApi.endpoints.me.matchRejected, (state, { payload }) => {
        if (payload?.status === 401) {
          state.user = null;
          state.status = "anonymous";
        }
      })
      .addMatcher(isAnyOf(authApi.endpoints.logout.matchFulfilled, sessionExpired), (state) => {
        state.user = null;
        state.status = "anonymous";
      });
  },
  selectors: {
    selectCurrentUser: (state) => state.user,
    selectAuthStatus: (state) => state.status,
  },
});

export const { selectCurrentUser, selectAuthStatus } = authSlice.selectors;
