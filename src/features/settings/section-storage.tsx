import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
}

export function SectionStorage({ settings }: Props) {
  const rows = [
    { label: "Recordings", value: settings.output_dir },
    { label: "Notes", value: settings.notes_dir },
    { label: "Transcripts", value: settings.transcripts_dir },
    { label: "Tasks", value: settings.tasks_path },
  ];

  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Storage</h2>
      <p className="text-sm text-muted-foreground">
        All paths are local. Folder pickers land in the next iteration.
      </p>
      <div className="grid gap-3">
        {rows.map((r) => (
          <div
            key={r.label}
            className="flex items-center justify-between gap-4 rounded-lg border border-border bg-card p-4"
          >
            <div>
              <p className="text-sm font-medium">{r.label}</p>
              <p className="mt-0.5 break-all font-mono text-xs text-muted-foreground">
                {r.value}
              </p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
