"use client";

import { ContactIcon, RefreshCwIcon, UsersIcon } from "lucide-react";

import { EmptyState } from "@/components/layout/EmptyState";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { selectCurrentUser } from "@/features/auth/authSlice";
import { errorMessage } from "@/lib/api-error";
import { useAppSelector } from "@/lib/store/hooks";

import { AddTeamMemberDialog } from "./AddTeamMemberDialog";
import { UsersTable } from "./UsersTable";
import { useListTeamQuery, useListUsersQuery } from "./usersApi";

function LoadError({ error, retry }: { error: Parameters<typeof errorMessage>[0]; retry: () => void }) {
  return (
    <EmptyState
      icon={RefreshCwIcon}
      title="We couldn't load this list"
      action={
        <Button variant="outline" onClick={retry}>
          Try again
        </Button>
      }
    >
      {errorMessage(error)}
    </EmptyState>
  );
}

export function TeamAndCustomers() {
  const me = useAppSelector(selectCurrentUser);
  const team = useListTeamQuery();
  const customers = useListUsersQuery({ role: "customer" });

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title="Team & customers"
        description="Add agents and admins, and see who has signed up to the customer portal."
        actions={<AddTeamMemberDialog />}
      />
      <Tabs defaultValue="team" className="gap-4">
        <TabsList>
          <TabsTrigger value="team">
            Team{team.data && <span className="text-faint tabular-nums">{team.data.length}</span>}
          </TabsTrigger>
          <TabsTrigger value="customers">
            Customers{customers.data && <span className="text-faint tabular-nums">{customers.data.length}</span>}
          </TabsTrigger>
        </TabsList>
        <TabsContent value="team">
          {team.isError ? (
            <LoadError error={team.error} retry={team.refetch} />
          ) : (
            <UsersTable
              users={team.data}
              isLoading={team.isLoading}
              currentUserId={me?.id}
              empty={<EmptyState icon={UsersIcon} title="No team members yet" />}
            />
          )}
        </TabsContent>
        <TabsContent value="customers">
          {customers.isError ? (
            <LoadError error={customers.error} retry={customers.refetch} />
          ) : (
            <UsersTable
              users={customers.data}
              isLoading={customers.isLoading}
              empty={
                <EmptyState icon={ContactIcon} title="No customers yet">
                  Customers appear here when they sign up to the portal or email your support address.
                </EmptyState>
              }
            />
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
