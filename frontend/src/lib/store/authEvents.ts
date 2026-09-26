import { createAction } from "@reduxjs/toolkit";

/** The refresh token was rejected: the user must sign in again. */
export const sessionExpired = createAction("auth/sessionExpired");
