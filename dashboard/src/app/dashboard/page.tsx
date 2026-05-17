import { redirect } from "next/navigation";

/**
 * /dashboard simply redirects to the workspace as the home view.
 */
export default function DashboardIndex() {
  redirect("/dashboard/workspace");
}
