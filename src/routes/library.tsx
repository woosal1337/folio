import { Library as LibraryIcon } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";

export default function Library() {
  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-10">
      <header data-drag="" className="select-none">
        <h1 className="font-serif text-3xl font-medium tracking-tight">
          Library
        </h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Every recording with its transcript inline.
        </p>
      </header>
      <Card>
        <CardContent className="flex flex-col items-center gap-3 py-16 text-center">
          <LibraryIcon className="h-8 w-8 text-muted-foreground" />
          <h2 className="font-medium">Library is empty</h2>
          <p className="max-w-sm text-sm text-muted-foreground">
            Start your first recording from the Record screen. It will appear
            here with playback and a one-click transcribe.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
