import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { RequestConversation } from "@/features/portal/RequestConversation";

export async function generateMetadata({ params }: PageProps<"/portal/requests/[number]">): Promise<Metadata> {
  const { number } = await params;
  return { title: `Request TKT-${number}` };
}

export default async function RequestPage({ params, searchParams }: PageProps<"/portal/requests/[number]">) {
  const number = Number((await params).number);
  if (!Number.isSafeInteger(number) || number <= 0) notFound();
  const { created } = await searchParams;
  return <RequestConversation number={number} fresh={created === "1"} />;
}
