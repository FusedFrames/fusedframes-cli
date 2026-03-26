export function outputSuccess(data: unknown): void {
  process.stdout.write(JSON.stringify(data) + "\n");
}

export function outputError(code: string, message: string): never {
  process.stdout.write(
    JSON.stringify({ error: { code, message } }) + "\n"
  );
  process.exit(1);
}
