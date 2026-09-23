export async function pickOverlaysDir(title: string): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title,
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickFile(opts: {
  title: string;
  defaultPath?: string | null;
  accept?: string[];
}): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const filters =
    opts.accept && opts.accept.length > 0
      ? [{ name: opts.title, extensions: opts.accept }]
      : undefined;
  const selected = await open({
    directory: false,
    multiple: false,
    title: opts.title,
    defaultPath: opts.defaultPath ?? undefined,
    filters,
  });
  return typeof selected === "string" ? selected : null;
}
