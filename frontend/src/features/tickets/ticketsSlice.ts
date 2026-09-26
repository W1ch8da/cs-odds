import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

import type { QueueView } from "./labels";
import type { Priority } from "./types";

type QueueState = {
  /** The queue view the agent last looked at, for "back" links. */
  lastView: QueueView;
  priority: Priority | "all";
  search: string;
  page: number;
};

const initialState: QueueState = { lastView: "mine", priority: "all", search: "", page: 1 };

/** Agent queue UI state. Ticket data itself lives in the RTK Query cache. */
export const ticketsSlice = createSlice({
  name: "ticketQueue",
  initialState,
  reducers: {
    viewOpened(state, action: PayloadAction<QueueView>) {
      if (state.lastView !== action.payload) state.page = 1;
      state.lastView = action.payload;
    },
    priorityFilterChanged(state, action: PayloadAction<Priority | "all">) {
      state.priority = action.payload;
      state.page = 1;
    },
    searchChanged(state, action: PayloadAction<string>) {
      state.search = action.payload;
      state.page = 1;
    },
    pageChanged(state, action: PayloadAction<number>) {
      state.page = action.payload;
    },
  },
  selectors: {
    selectQueue: (state) => state,
  },
});

export const { viewOpened, priorityFilterChanged, searchChanged, pageChanged } = ticketsSlice.actions;
export const { selectQueue } = ticketsSlice.selectors;
