import { redirect } from "next/navigation";

/**
 * Root route — always sends visitors to the workspace. Auth middleware
 * will bounce unauthenticated requests to `/login`.
 */
export default function RootPage() {
  redirect("/dashboard/workspace");
}
