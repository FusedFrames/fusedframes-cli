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

// fetch() rejects with an opaque "fetch failed" on any network-level problem
// (DNS, refused connection, TLS, timeout). Surface the real reason and where to
// look so the user isn't left guessing.
function describeNetworkError(err: unknown, baseUrl: string): string {
  if (err instanceof Error && err.name === "TimeoutError") {
    return `Request to ${baseUrl} timed out after 30s. Check your connection, or the FUSEDFRAMES_API_URL setting.`;
  }
  // Node puts the underlying reason (ECONNREFUSED, ENOTFOUND, ...) on err.cause.
  const cause = err instanceof Error ? err.cause : undefined;
  const code =
    cause && typeof cause === "object" && "code" in cause
      ? String((cause as { code?: unknown }).code)
      : undefined;
  const reason =
    code ??
    (cause instanceof Error ? cause.message : undefined) ??
    (err instanceof Error ? err.message : String(err));
  return `Could not reach the FusedFrames API at ${baseUrl} (${reason}). Check your internet connection and the FUSEDFRAMES_API_URL setting.`;
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

  let response: Response;
  try {
    response = await fetch(url.toString(), {
      method: "GET",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        Accept: "application/json",
        "User-Agent": `@fusedframes/cli/${VERSION}`,
      },
      signal: AbortSignal.timeout(30_000),
    });
  } catch (err) {
    throw new FusedFramesError("network_error", describeNetworkError(err, baseUrl));
  }

  if (!response.ok) {
    const rawBody = await response.text().catch(() => "");
    let parsed: ApiError | undefined;
    if (rawBody) {
      try {
        parsed = JSON.parse(rawBody) as ApiError;
      } catch {
        parsed = undefined;
      }
    }

    // A well-formed API error: pass its code and message straight through.
    if (parsed?.error?.code || parsed?.error?.message) {
      throw new FusedFramesError(
        parsed.error?.code || "unknown",
        parsed.error?.message || `HTTP ${response.status}`
      );
    }

    // Non-JSON error body (HTML error page, plain text, gateway error, empty):
    // surface the raw body instead of collapsing it to just "HTTP <status>".
    const detail = rawBody.trim().slice(0, 500);
    let message = detail ? `HTTP ${response.status}: ${detail}` : `HTTP ${response.status}`;
    if (response.status === 404) {
      message +=
        " The API may have changed — update the CLI with `npm i -g @fusedframes/cli`.";
    }
    throw new FusedFramesError("server_error", message);
  }

  return (await response.json()) as T;
}
