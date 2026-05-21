import { Mic } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

export default function Record() {
  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-10">
      <header className="flex items-baseline justify-between">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">
            Record
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Capture system audio and microphone independently.
          </p>
        </div>
        <Badge variant="outline" className="font-mono tracking-tight">
          <span className="mr-2 inline-block h-2 w-2 rounded-full border border-muted-foreground" />
          idle · 00:00
        </Badge>
      </header>

      <Card>
        <CardContent className="flex flex-col items-center gap-4 py-12">
          <Button size="xl" className="w-full max-w-md gap-3">
            <Mic className="h-5 w-5" />
            Start recording
          </Button>
          <p className="text-xs text-muted-foreground">
            Mic + system audio in parallel · OpenAI transcription on stop
          </p>
        </CardContent>
      </Card>

      <div>
        <h2 className="font-medium text-foreground">Recent recordings</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Your sessions will appear here once you start recording.
        </p>
      </div>
    </div>
  );
}
