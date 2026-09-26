"use client";

import { setupListeners } from "@reduxjs/toolkit/query";
import { useEffect, useState } from "react";
import { Provider } from "react-redux";

import { makeStore } from "./store";

export function StoreProvider({ children }: { children: React.ReactNode }) {
  // Lazy initialiser: one store per browser session / server request.
  const [store] = useState(makeStore);

  // Enables refetchOnFocus / refetchOnReconnect for RTK Query.
  useEffect(() => setupListeners(store.dispatch), [store]);

  return <Provider store={store}>{children}</Provider>;
}
