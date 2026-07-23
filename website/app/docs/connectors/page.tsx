import type { Metadata } from "next";
import Link from "next/link";
import { siteConfig } from "@/lib/site-config";
import { CodeBlock } from "@/components/site/code-block";
import {
    DocHeader,
    DocH2,
    Prose,
    Callout,
    Steps,
    Step,
    FieldList,
    Field,
} from "@/components/docs/doc-primitives";

export const metadata: Metadata = {
    title: "Connectors (MCP)",
    description:
        "Expose your transcripts, tasks, and memory to MCP-aware tools over a local stdio server, with nothing sent to a third party.",
};

const mcpConfig = `{
  "mcpServers": {
    "folio": {
      "command": "folio-mcp",
      "args": []
    }
  }
}`;

export default function ConnectorsPage() {
    return (
        <>
            <DocHeader
                eyebrow="Going deeper"
                title="Connectors"
                description={`${siteConfig.name} exposes your transcripts, tasks, and memory to MCP-aware tools, locally. Access is read-only, runs over stdio, and never touches the cloud.`}
            />

            <DocH2 id="what-is-mcp">What is MCP</DocH2>
            <Prose>
                <p>
                    The Model Context Protocol is an open standard that lets an
                    application expose context and tools to an AI client through a small,
                    well-defined interface. A client connects to a server, lists the tools
                    it offers, and calls them on demand. The protocol does not require a
                    network. A server can run as a local process and speak to the client
                    over standard input and output.
                </p>
                <p>
                    {siteConfig.name} ships a local MCP server named{" "}
                    <code>folio-mcp</code>. It gives any MCP-aware tool{" "}
                    <strong>read-only</strong> access to your transcripts, tasks, and
                    memories over <code>stdio</code>. There is no cloud and no proxy. The
                    server runs on your machine, reads from your vault, and answers the
                    client directly. The one exception is <code>create_task</code>, which
                    writes a new task back into your vault.
                </p>
            </Prose>

            <DocH2 id="setup">Setup</DocH2>
            <Prose>
                <p>
                    Enabling connectors takes two steps. Turn the server on inside{" "}
                    {siteConfig.name}, then point your MCP client at it.
                </p>
            </Prose>
            <Steps>
                <Step n={1} title="Enable connectors in Folio">
                    <p>
                        Open <strong>Settings</strong> then <strong>Connectors</strong>{" "}
                        and enable the local MCP server. {siteConfig.name} runs{" "}
                        <code>folio-mcp</code> on your machine and serves it over{" "}
                        <code>stdio</code>.
                    </p>
                </Step>
                <Step n={2} title="Register folio-mcp with your client">
                    <p>
                        Add the server to your MCP client configuration. The client
                        launches <code>folio-mcp</code> as a subprocess and talks to it
                        over standard input and output. A minimal configuration registers
                        a single server named <code>folio</code> with the command{" "}
                        <code>folio-mcp</code> and an empty argument list.
                    </p>
                    <CodeBlock code={mcpConfig} label="mcp.json" />
                </Step>
            </Steps>

            <DocH2 id="compatible-tools">Compatible tools</DocH2>
            <Prose>
                <p>
                    Any tool that speaks the Model Context Protocol can connect to{" "}
                    <code>folio-mcp</code>. That includes the clients people most often
                    reach for.
                </p>
                <ul>
                    <li>Claude Desktop</li>
                    <li>Cursor</li>
                    <li>Claude Code</li>
                    <li>Any MCP client</li>
                </ul>
                <p>
                    The configuration shape varies slightly between clients, but the
                    server entry is the same. Register <code>folio</code> with the command{" "}
                    <code>folio-mcp</code> and no arguments.
                </p>
            </Prose>

            <DocH2 id="available-tools">Available tools</DocH2>
            <Prose>
                <p>
                    The server exposes a focused set of tools over MCP. Every tool reads
                    from your local vault. Only <code>create_task</code> writes.
                </p>
            </Prose>
            <FieldList>
                <Field name="search_memory" type="read">
                    Searches your stored memories and returns the entries that match a
                    query.
                </Field>
                <Field name="recent_meetings" type="read">
                    Lists your most recent meetings, newest first.
                </Field>
                <Field name="get_transcript" type="read">
                    Returns the full transcript for a single meeting.
                </Field>
                <Field name="notes_by_date_range" type="read">
                    Returns the notes that fall within a start and end date.
                </Field>
                <Field name="notes_by_folder" type="read">
                    Returns the notes stored under a given folder in your vault.
                </Field>
                <Field name="notes_by_person" type="read">
                    Returns the notes that involve a specific attendee.
                </Field>
                <Field name="quote_segment" type="read">
                    Returns an exact quoted segment from a transcript with its speaker
                    attribution.
                </Field>
                <Field name="find_decision" type="read">
                    Searches transcripts for a decision and returns where it was made.
                </Field>
                <Field name="list_tasks" type="read">
                    Lists the tasks held in your vault.
                </Field>
                <Field name="create_task" type="write">
                    Creates a new task and writes it back into your vault. This is the one
                    write-style tool the server exposes.
                </Field>
            </FieldList>

            <DocH2 id="privacy-and-scope">Privacy and scope</DocH2>
            <Callout variant="privacy" title="Local and read-only by design">
                <p>
                    Access runs over <code>stdio</code> and is read-only, with{" "}
                    <code>create_task</code> as the only exception. The server is scoped
                    to your local vault. Nothing is sent to a third party, and no cloud or
                    proxy sits between your client and your notes. For the full picture of
                    what stays on your machine, see the{" "}
                    <Link href="/docs/privacy">privacy documentation</Link>.
                </p>
            </Callout>
        </>
    );
}
