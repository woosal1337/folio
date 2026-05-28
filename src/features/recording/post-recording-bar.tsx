/**
 * GET-146 — post-recording action bar.
 *
 * After Stop, Granola surfaces an action bar rather than silently
 * auto-running agents. The primary action is Generate notes (runs the
 * transcription → summary pipeline); the rest — Resume, Ask anything,
 * Write follow-up email — land in their own issues (GET-149 / GET-150 /
 * GET-147) and light up here as each ships.
 */

import * as React from "react";
import { Loader2, MessageCircleQuestion, Play, Sparkles } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { FollowupEmailButton } from "@/features/recording/followup-email-button";
import { NoteChat } from "@/features/recording/note-chat";

interface Props {
  /** Session directory of the just-stopped recording. */
  sessionDir: string;
  /** True while the generate pipeline is running for this session. */
  generating: boolean;
  /** True once a transcript has been produced for this session. */
  generated: boolean;
  /** Run the transcription → summary pipeline. */
  onGenerate: () => void;
}

export function PostRecordingBar({
  sessionDir,
  generating,
  generated,
  onGenerate,
}: Props) {
  const label = basename(sessionDir);
  const [chatOpen, setChatOpen] = React.useState(false);
  return (
    <>
      <Card>
        <CardContent className="flex flex-col gap-3 py-4">
          <div className="flex items-baseline justify-between">
            <p className="text-sm font-medium text-foreground">Recording saved</p>
            <p className="truncate pl-3 text-xs text-muted-foreground">{label}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              className="gap-2"
              onClick={onGenerate}
              disabled={generating}
              aria-busy={generating}
            >
              {generating ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Sparkles className="h-4 w-4" />
              )}
              {generating
                ? "Generating notes…"
                : generated
                  ? "Regenerate notes"
                  : "Generate notes"}
            </Button>

            {/* Resume lands in GET-149's record-route controls; here it
              stays a hint. */}
            <Button
              variant="outline"
              className="gap-2"
              disabled
              title="Use Pause while recording to resume into the same note (GET-149)"
            >
              <Play className="h-4 w-4" />
              Resume
            </Button>
            <Button
              variant="outline"
              className="gap-2"
              onClick={() => setChatOpen(true)}
              disabled={!generated}
              title={
                generated
                  ? "Ask anything about this meeting"
                  : "Generate notes first so there's a transcript to ask about"
              }
            >
              <MessageCircleQuestion className="h-4 w-4" />
              Ask anything
            </Button>
            <FollowupEmailButton sessionDir={sessionDir} disabled={!generated} />
          </div>
        </CardContent>
      </Card>
      <NoteChat
        sessionDir={sessionDir}
        open={chatOpen}
        onOpenChange={setChatOpen}
        seed="What did I miss?"
      />
    </>
  );
}

function basename(p: string): string {
  const parts = p.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? p;
}
