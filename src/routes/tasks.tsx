import { KanbanSquare } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";

export default function Tasks() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-8 py-10">
      <header data-drag="" className="select-none">
        <h1 className="font-serif text-3xl font-medium tracking-tight">
          Tasks
        </h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Trello-style kanban for the work tied to your meetings.
        </p>
      </header>
      <Card>
        <CardContent className="flex flex-col items-center gap-3 py-16 text-center">
          <KanbanSquare className="h-8 w-8 text-muted-foreground" />
          <h2 className="font-medium">Drag-and-drop kanban coming next session</h2>
          <p className="max-w-sm text-sm text-muted-foreground">
            Three columns (To-do, Doing, Done). Pick up a card and drop it
            anywhere. Click to edit.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
