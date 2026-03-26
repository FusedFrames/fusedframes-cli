import { requireApiKey, getApiUrl } from "./config.js";

interface ApiError {
  error: { code: string; message: string };
}

export class FusedFramesError extends Error {
  code: string;
  status: number;

  constructor(code: string, message: string, status: number) {
    super(message);
    this.code = code;
    this.status = status;
  }
}

export async function request<T>(
  path: string,
  params?: Record<string, string | undefined>
): Promise<T> {
  const apiKey = requireApiKey();
  const baseUrl = getApiUrl();

  // Enforce HTTPS (allow http://localhost for dev)
  if (!baseUrl.startsWith("https://") && !baseUrl.startsWith("http://localhost")) {
    throw new FusedFramesError(
      "config_error",
      "API URL must use HTTPS. API keys cannot be sent over unencrypted connections.",
      0
    );
  }

  // Build URL with query params
  const url = new URL(path, baseUrl);
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined && value !== "") {
        url.searchParams.set(key, value);
      }
    }
  }

  const response = await fetch(url.toString(), {
    method: "GET",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      Accept: "application/json",
      "User-Agent": "@fusedframes/cli/1.0.0",
    },
    signal: AbortSignal.timeout(30_000),
  });

  if (!response.ok) {
    let errorBody: ApiError;
    try {
      errorBody = (await response.json()) as ApiError;
    } catch {
      throw new FusedFramesError(
        "server_error",
        `HTTP ${response.status}`,
        response.status
      );
    }
    throw new FusedFramesError(
      errorBody.error?.code || "unknown",
      errorBody.error?.message || `HTTP ${response.status}`,
      response.status
    );
  }

  return (await response.json()) as T;
}
