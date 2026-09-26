import type { Metadata } from "next";

import { TicketQueue } from "@/features/tickets/components/TicketQueue";
import { isQueueView } from "@/features/tickets/labels";

export const metadata: Metadata = { title: "Tickets" };

export default async function TicketsPage({ searchParams }: PageProps<"/agent/tickets">) {
  const { view } = await searchParams;
  return <TicketQueue view={isQueueView(view) ? view : "mine"} />;
}
