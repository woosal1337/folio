import { open, showHUD } from "@raycast/api";

export default async function StartRecording() {
  await open("folio://record?autoStart=1");
  await showHUD("Folio: starting recording");
}
