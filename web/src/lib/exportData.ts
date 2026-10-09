// The data export: what export_my_data() returns, plus the bytes of each of
// the user's game files (gzipped JSON, as stored), in one JSON document.

export interface ExportFile {
  name: string;
  bytes: Uint8Array;
}

type ExportDoc = Record<string, unknown> & { files: unknown[] };

function asExport(data: unknown): ExportDoc & { account: { id: string } } {
  const d = data as Record<string, unknown> | null;
  const account = d?.account as { id?: unknown } | null | undefined;
  if (
    !d || d.format !== "tavern-ledger-export" || !Array.isArray(d.files) ||
    typeof account?.id !== "string"
  ) {
    throw new Error("the server did not return an export");
  }
  return d as ExportDoc & { account: { id: string } };
}

/** Names of the user's own files listed in the export, to download them. */
export function ownFileNames(data: unknown): string[] {
  const doc = asExport(data);
  const prefix = `${doc.account.id}/`;
  return doc.files.flatMap((f) => {
    const file = f as { bucket?: unknown; name?: unknown } | null;
    if (file?.bucket !== "games" || typeof file.name !== "string") return [];
    const rest = file.name.slice(prefix.length);
    return file.name.startsWith(prefix) && /^[A-Za-z0-9_.-]+$/.test(rest) && !rest.includes("..")
      ? [file.name]
      : [];
  });
}

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(binary);
}

export function buildExport(data: unknown, files: readonly ExportFile[]): Record<string, unknown> {
  const doc = asExport(data);
  return {
    ...doc,
    file_contents: files.map((f) => ({
      name: f.name,
      encoding: "gzip+base64",
      data: bytesToBase64(f.bytes),
    })),
  };
}

export function exportFileName(now: Date): string {
  return `tavern-ledger-export-${now.toISOString().slice(0, 10)}.json`;
}
