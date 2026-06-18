import Link from "next/link";
import { ArrowRight } from "lucide-react";

import { siteConfig } from "@/lib/site-config";
import { Section, SectionHeading } from "@/components/site/section";
import { Button } from "@/components/ui/button";
import { CodeBlock } from "@/components/site/code-block";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

const homebrew = `${siteConfig.install.tapCommand}
${siteConfig.install.installCommand}`;

const source = `git clone ${siteConfig.links.github}.git
cd folio
bun install
bun tauri dev`;

export function InstallSection() {
  return (
    <Section>
      <div className="mx-auto max-w-3xl">
        <SectionHeading
          align="center"
          eyebrow="Install"
          title="Up and running in a minute"
          description={`Requires ${siteConfig.platform} on Apple Silicon or Intel. The first run downloads the model weights once, then everything is local.`}
        />

        <Tabs defaultValue="homebrew" className="mt-12 flex flex-col items-center">
          <TabsList>
            <TabsTrigger value="homebrew">Homebrew</TabsTrigger>
            <TabsTrigger value="dmg">Direct download</TabsTrigger>
            <TabsTrigger value="source">From source</TabsTrigger>
          </TabsList>

          <div className="mt-5 w-full">
            <TabsContent value="homebrew">
              <CodeBlock code={homebrew} label="Recommended" />
            </TabsContent>
            <TabsContent value="dmg">
              <div className="flex flex-col gap-4 rounded-lg border border-border bg-card p-6 shadow-sm">
                <p className="text-ms-15 leading-relaxed text-muted-foreground">
                  Grab the latest Apple Silicon <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-2xs">.dmg</code>{" "}
                  from the releases page, open it, and drag Folio to Applications. Builds
                  are code-signed and notarized, so they open without a Gatekeeper prompt.
                </p>
                <Button asChild variant="outline" className="w-fit">
                  <Link href={siteConfig.links.releases} target="_blank" rel="noreferrer">
                    Download the latest release
                    <ArrowRight className="h-4 w-4" />
                  </Link>
                </Button>
              </div>
            </TabsContent>
            <TabsContent value="source">
              <CodeBlock code={source} label="Build from source" />
            </TabsContent>
          </div>
        </Tabs>

        <p className="mt-6 text-center text-2xs text-muted-foreground">
          Already installed? Update any time with{" "}
          <code className="rounded bg-muted px-1.5 py-0.5 font-mono">
            {siteConfig.install.upgradeCommand}
          </code>
          .
        </p>
      </div>
    </Section>
  );
}
