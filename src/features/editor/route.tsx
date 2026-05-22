import { Pencil } from "lucide-react";
import { Card, CardContent } from "@/shared/ui/card";

export default function Editor() {
  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-10">
      <header data-drag="" className="select-none">
        <h1 className="font-serif text-3xl font-medium tracking-tight">Editor</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Notion-style notes alongside your meetings.
        </p>
      </header>
      <Card>
        <CardContent className="flex flex-col items-center gap-3 py-16 text-center">
          <Pencil className="h-8 w-8 text-muted-foreground" />
          <h2 className="font-medium">TipTap editor coming next session</h2>
          <p className="max-w-sm text-sm text-muted-foreground">
            File rail on the left with a + button. Click a note to open it full-width in
            the right pane. Autosave on every keystroke.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
