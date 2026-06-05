import { open, showHUD } from "@raycast/api";

export default async function StartRecording() {
  await open("attune://record?autoStart=1");
  await showHUD("Attune: starting recording");
}
