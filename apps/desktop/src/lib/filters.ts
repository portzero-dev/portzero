import type { RequestSummary, RequestFilters } from "./types";

/**
 * Apply client-side filters to an array of request summaries.
 * This complements server-side filtering for instant UI filtering.
 */
export function filterRequests(
  requests: RequestSummary[],
  filters: RequestFilters,
): RequestSummary[] {
  let result = requests;

  if (filters.app) {
    result = result.filter((r) => r.app_name === filters.app);
  }

  if (filters.method) {
    result = result.filter(
      (r) => r.method.toUpperCase() === filters.method!.toUpperCase(),
    );
  }

  if (filters.status !== undefined) {
    result = result.filter((r) => r.status_code === filters.status);
  }

  if (filters.status_range) {
    const range = filters.status_range;
    result = result.filter((r) => {
      if (range === "2xx") return r.status_code >= 200 && r.status_code < 300;
      if (range === "3xx") return r.status_code >= 300 && r.status_code < 400;
      if (range === "4xx") return r.status_code >= 400 && r.status_code < 500;
      if (range === "5xx") return r.status_code >= 500 && r.status_code < 600;
      return true;
    });
  }

  if (filters.path) {
    const pathLower = filters.path.toLowerCase();
    result = result.filter((r) => r.path.toLowerCase().includes(pathLower));
  }

  if (filters.search) {
    const searchLower = filters.search.toLowerCase();
    result = result.filter(
      (r) =>
        r.path.toLowerCase().includes(searchLower) ||
        r.method.toLowerCase().includes(searchLower) ||
        r.app_name.toLowerCase().includes(searchLower) ||
        String(r.status_code).includes(searchLower),
    );
  }

  return result;
}

/**
 * Available HTTP methods for filter dropdown.
 */
export const HTTP_METHODS = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
] as const;

/**
 * Available status ranges for filter dropdown.
 */
export const STATUS_RANGES = [
  { value: "", label: "All statuses" },
  { value: "2xx", label: "2xx Success" },
  { value: "3xx", label: "3xx Redirect" },
  { value: "4xx", label: "4xx Client Error" },
  { value: "5xx", label: "5xx Server Error" },
] as const;
