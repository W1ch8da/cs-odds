import { baseApi } from "@/lib/store/api";

import type {
  AddCommentRequest,
  CreateTicketRequest,
  ListTicketsParams,
  Page,
  TicketDetail,
  TicketSummary,
  UpdateTicketRequest,
  ViewCounts,
} from "./types";

const LIST = { type: "Ticket", id: "LIST" } as const;
const COUNTS = { type: "Ticket", id: "COUNTS" } as const;

/** Every write returns the full ticket: store it, and refresh lists and counts. */
async function storeTicket(
  _arg: unknown,
  {
    dispatch,
    queryFulfilled,
  }: { dispatch: (action: unknown) => unknown; queryFulfilled: Promise<{ data: TicketDetail }> },
) {
  try {
    const { data } = await queryFulfilled;
    dispatch(ticketsApi.util.upsertQueryData("getTicket", data.number, data));
  } catch {
    // The calling component shows the error.
  }
}

export const ticketsApi = baseApi.injectEndpoints({
  endpoints: (build) => ({
    listTickets: build.query<Page<TicketSummary>, ListTicketsParams>({
      query: (params) => ({ url: "/tickets", params }),
      providesTags: [LIST],
    }),
    ticketCounts: build.query<ViewCounts, void>({
      query: () => "/tickets/counts",
      providesTags: [COUNTS],
    }),
    getTicket: build.query<TicketDetail, number>({
      query: (number) => `/tickets/${number}`,
      providesTags: (_res, _err, number) => [{ type: "Ticket", id: number }],
    }),
    createTicket: build.mutation<TicketDetail, CreateTicketRequest>({
      query: (body) => ({ url: "/tickets", method: "POST", body }),
      invalidatesTags: [LIST, COUNTS],
      onQueryStarted: storeTicket,
    }),
    updateTicket: build.mutation<TicketDetail, UpdateTicketRequest>({
      query: ({ number, ...body }) => ({ url: `/tickets/${number}`, method: "PATCH", body }),
      invalidatesTags: [LIST, COUNTS],
      onQueryStarted: storeTicket,
    }),
    addComment: build.mutation<TicketDetail, AddCommentRequest>({
      query: ({ number, ...body }) => ({ url: `/tickets/${number}/comments`, method: "POST", body }),
      invalidatesTags: [LIST, COUNTS],
      onQueryStarted: storeTicket,
    }),
  }),
});

export const {
  useListTicketsQuery,
  useTicketCountsQuery,
  useGetTicketQuery,
  useCreateTicketMutation,
  useUpdateTicketMutation,
  useAddCommentMutation,
} = ticketsApi;
