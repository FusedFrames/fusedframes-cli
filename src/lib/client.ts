import { requireApiKey, getApiUrl } from "./config.js";
import { VERSION } from "./version.js";

interface ApiError {
  error: { code: string; message: string };
}

export class FusedFramesError extends Error {
  code: string;

  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

export async function request<T>(
  path: string,
  params?: Record<string, string | undefined>
): Promise<T> {
  const apiKey = requireApiKey();
  const baseUrl = getApiUrl();

  // Require HTTPS so the API key is never sent in clear text. A plain-http
  // exemption is allowed ONLY for genuine loopback hosts. Parse the URL and
  // match the host exactly: a prefix check such as
  // `startsWith("http://localhost")` also accepts `http://localhost.evil.com`
  // and `http://localhost@evil.com`, which would leak the bearer key to an
  // attacker-controlled host.
  let parsedBase: URL;
  try {
    parsedBase = new URL(baseUrl);
  } catch {
    throw new FusedFramesError(
      "config_error",
      "API URL is not a valid URL."
    );
  }

  const isLoopback =
    parsedBase.protocol === "http:" &&
    (parsedBase.hostname === "localhost" ||
      parsedBase.hostname === "127.0.0.1" ||
      parsedBase.hostname === "::1" ||
      parsedBase.hostname === "[::1]");

  if (parsedBase.protocol !== "https:" && !isLoopback) {
    throw new FusedFramesError(
      "config_error",
      "API URL must use HTTPS. API keys cannot be sent over unencrypted connections."
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
      "User-Agent": `@fusedframes/cli/${VERSION}`,
    },
    signal: AbortSignal.timeout(30_000),
  });

  if (!response.ok) {
    let errorBody: ApiError;
    try {
      errorBody = (await response.json()) as ApiError;
    } catch {
      throw new FusedFramesError("server_error", `HTTP ${response.status}`);
    }
    throw new FusedFramesError(
      errorBody.error?.code || "unknown",
      errorBody.error?.message || `HTTP ${response.status}`
    );
  }

  return (await response.json()) as T;
}
