/**
 * Turkish message table — first translated bundle riding on the i18n
 * foundation GET-118 shipped. Keys mirror messages.en.ts exactly so a
 * future locale switcher can swap one for the other with no missing
 * entries.
 *
 * v2 roadmap finding 085 / GET-105.
 */

export const messagesTr = {
  // ---- Library surface ------------------------------------------
  "errors.recordings.load": "Kayıtlar yüklenemedi",
  "errors.recording.delete": "Kayıt silinemedi",
  "errors.recording.reveal": "Finder açılamadı",
  "errors.recording.load": "Kayıt yüklenemedi",

  // ---- Transcription surface ------------------------------------
  "errors.transcription.start": "Yazıya dönüştürme başlatılamadı",
  "errors.transcription.retry": "Yeniden yazıya dönüştürülemedi",
  "errors.transcript.save": "Transcript kaydedilemedi",
  "errors.transcript.export": "Transcript dışa aktarılamadı",

  // ---- Agents surface -------------------------------------------
  "errors.agents.load": "Ajanlar yüklenemedi",
  "errors.agents.run": "{name} başarısız oldu",
  "errors.agents.deleteResult": "{name} sonucu silinemedi",
  "errors.agents.runs.load": "AI çalıştırmaları yüklenemedi",

  // ---- Settings surface -----------------------------------------
  "errors.settings.load": "Ayarlar yüklenemedi",
  "errors.settings.save": "Ayarlar kaydedilemedi",

  // ---- Webhooks surface -----------------------------------------
  "errors.webhooks.load": "Web kancaları yüklenemedi",
  "errors.webhooks.save": "Web kancası kaydedilemedi",
  "errors.webhooks.delete": "Web kancası silinemedi",
  "errors.webhooks.test": "Web kancası testi başarısız",
  "errors.webhooks.update": "Web kancası güncellenemedi",

  // ---- Snapshot / storage ---------------------------------------
  "errors.snapshot.export": "Anlık görüntü dışa aktarılamadı",

  // ---- Usage / costs --------------------------------------------
  "errors.usage.load": "Kullanım verileri yüklenemedi",

  // ---- Memory / share -------------------------------------------
  "errors.share.openObsidian": "Obsidian'da açılamadı",
  "errors.share.copy": "Kopyalanamadı",

  // ---- Generic --------------------------------------------------
  "errors.generic": "Bir şeyler ters gitti",
} as const;
