import { combineSlices, configureStore } from "@reduxjs/toolkit";

import { authSlice } from "@/features/auth/authSlice";
import { ticketsSlice } from "@/features/tickets/ticketsSlice";

import { baseApi } from "./api";

const rootReducer = combineSlices(baseApi, authSlice, ticketsSlice);

/**
 * Creates a new store. With the App Router the store must be created per
 * request (inside StoreProvider), never as a module-level singleton, so state
 * is not shared between users during server rendering.
 */
export function makeStore() {
  return configureStore({
    reducer: rootReducer,
    middleware: (getDefaultMiddleware) => getDefaultMiddleware().concat(baseApi.middleware),
  });
}

export type AppStore = ReturnType<typeof makeStore>;
export type RootState = ReturnType<typeof rootReducer>;
export type AppDispatch = AppStore["dispatch"];
