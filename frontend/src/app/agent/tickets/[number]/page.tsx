import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { TicketWorkspace } from "@/features/tickets/components/TicketWorkspace";

export async function generateMetadata({ params }: PageProps<"/agent/tickets/[number]">): Promise<Metadata> {
  const { number } = await params;
  return { title: `TKT-${number}` };
}

export default async function TicketPage({ params }: PageProps<"/agent/tickets/[number]">) {
  const number = Number((await params).number);
  if (!Number.isSafeInteger(number) || number <= 0) notFound();
  return <TicketWorkspace number={number} />;
}
