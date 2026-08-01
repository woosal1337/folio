import { humanizeError } from "@/shared/lib/errors";
import { toast } from "sonner";

export async function copyToClipboard(text: string, label = "Copied"): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast.success(label);
  } catch (e) {
    console.error("copyToClipboard:", e);
    toast.error("Could not copy", { description: humanizeError(e) });
  }
}
