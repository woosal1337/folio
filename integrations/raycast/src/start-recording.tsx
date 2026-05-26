import { open, showHUD } from "@raycast/api";

/**
 * Trigger Attune's recording flow via the registered `attune://`
 * deep link. The app's DeepLinkHandler reads the URL and routes to
 * /record with autoStart=true. v2 finding 087 / GET-69 introduced
 * the deep link scheme; this is the Raycast surface for it.
 */
export default async function StartRecording() {
  await open("attune://record?autoStart=1");
  await showHUD("Attune: starting recording");
}
