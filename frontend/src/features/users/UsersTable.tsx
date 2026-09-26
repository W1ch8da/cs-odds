import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { ROLE_LABELS, initials } from "@/features/auth/roles";
import type { User } from "@/features/auth/types";
import { formatDate } from "@/lib/format";

const ROLE_BADGE = { admin: "default", agent: "secondary", customer: "outline" } as const;

type Props = { users: User[] | undefined; isLoading: boolean; currentUserId?: string; empty: React.ReactNode };

export function UsersTable({ users, isLoading, currentUserId, empty }: Props) {
  if (!isLoading && users?.length === 0) return empty;

  return (
    <div className="overflow-x-auto rounded-xl border bg-card">
      <Table>
        <TableHeader>
          <TableRow className="bg-muted/60 hover:bg-muted/60">
            <TableHead className="pl-4">Name</TableHead>
            <TableHead>Role</TableHead>
            <TableHead className="pr-4 text-right">Joined</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {isLoading
            ? Array.from({ length: 3 }, (_, i) => (
                <TableRow key={i}>
                  <TableCell className="pl-4">
                    <Skeleton className="h-9 w-56" />
                  </TableCell>
                  <TableCell>
                    <Skeleton className="h-5 w-16" />
                  </TableCell>
                  <TableCell className="pr-4">
                    <Skeleton className="ml-auto h-5 w-20" />
                  </TableCell>
                </TableRow>
              ))
            : users?.map((user) => (
                <TableRow key={user.id}>
                  <TableCell className="pl-4">
                    <div className="flex items-center gap-3">
                      <Avatar className="size-8">
                        <AvatarFallback className="text-xs font-semibold">{initials(user.name)}</AvatarFallback>
                      </Avatar>
                      <div className="min-w-0 leading-tight">
                        <p className="truncate font-medium">
                          {user.name}
                          {user.id === currentUserId && <span className="ml-1.5 font-normal text-faint">(you)</span>}
                        </p>
                        <p className="truncate text-muted-foreground">{user.email}</p>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant={ROLE_BADGE[user.role]}>{ROLE_LABELS[user.role]}</Badge>
                  </TableCell>
                  <TableCell className="pr-4 text-right text-muted-foreground tabular-nums">
                    {formatDate(user.createdAt)}
                  </TableCell>
                </TableRow>
              ))}
        </TableBody>
      </Table>
    </div>
  );
}
