import type { Role, User } from "@/features/auth/types";
import { baseApi } from "@/lib/store/api";

export type CreateTeamMemberRequest = {
  name: string;
  email: string;
  password: string;
  role: Extract<Role, "agent" | "admin">;
};

export const usersApi = baseApi.injectEndpoints({
  endpoints: (build) => ({
    /** Admin only. */
    listUsers: build.query<User[], { role?: Role } | void>({
      query: (args) => ({ url: "/users", params: args?.role ? { role: args.role } : undefined }),
      providesTags: [{ type: "User", id: "LIST" }],
    }),
    /** Agents and admins: everyone a ticket can be assigned to. */
    listTeam: build.query<User[], void>({
      query: () => "/agents",
      providesTags: [{ type: "User", id: "LIST" }],
    }),
    createTeamMember: build.mutation<User, CreateTeamMemberRequest>({
      query: (body) => ({ url: "/users", method: "POST", body }),
      invalidatesTags: [{ type: "User", id: "LIST" }],
    }),
  }),
});

export const { useListUsersQuery, useListTeamQuery, useCreateTeamMemberMutation } = usersApi;
