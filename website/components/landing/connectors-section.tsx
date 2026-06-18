import Link from "next/link";
import { ArrowRight, Terminal } from "lucide-react";

import { Section, SectionHeading } from "@/components/site/section";
import { Button } from "@/components/ui/button";
import { CodeBlock } from "@/components/site/code-block";

const tools = ["Claude Desktop", "Cursor", "Claude Code", "Any MCP client"];

const mcpConfig = `{
  "mcpServers": {
    "folio": {
      "command": "folio-mcp",
      "args": []
    }
  }
}`;

export function ConnectorsSection() {
  return (
    <Section className="bg-secondary/40">
      <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
        <div className="order-2 lg:order-1">
          <CodeBlock code={mcpConfig} label="mcp.json" />
          <div className="mt-4 flex flex-wrap gap-2">
            {tools.map((tool) => (
              <span
                key={tool}
                className="inline-flex items-center gap-1.5 rounded-full border border-border bg-card px-3 py-1 text-2xs font-medium text-muted-foreground"
              >
                <Terminal className="h-3 w-3 text-primary" />
                {tool}
              </span>
            ))}
          </div>
        </div>

        <div className="order-1 flex flex-col gap-6 lg:order-2">
          <SectionHeading
            eyebrow="Connectors"
            title="Your transcripts, available to your agents"
            description="Folio ships a local MCP server. Any MCP-aware tool gets read-only access to your transcripts, tasks, and memory over stdio — no cloud, no proxy."
          />
          <ul className="flex flex-col gap-3 text-ms-15 leading-relaxed text-muted-foreground">
            <li>Search past meetings without leaving your editor.</li>
            <li>Pull decisions and action items into your workflow.</li>
            <li>Everything runs on stdio, scoped to read-only.</li>
          </ul>
          <Button asChild variant="outline" className="w-fit">
            <Link href="/docs/connectors">
              Set up connectors
              <ArrowRight className="h-4 w-4" />
            </Link>
          </Button>
        </div>
      </div>
    </Section>
  );
}
