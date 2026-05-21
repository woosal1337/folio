import * as React from "react";
import {
  HashRouter,
  Route,
  Routes,
  Navigate,
} from "react-router-dom";

import { Sidebar } from "@/components/sidebar";
import { DragStrip } from "@/components/drag-strip";
import Record from "@/routes/record";
import Library from "@/routes/library";
import Editor from "@/routes/editor";
import Tasks from "@/routes/tasks";
import { SettingsModal } from "@/routes/settings-modal";

export default function App() {
  const [settingsOpen, setSettingsOpen] = React.useState(false);

  return (
    <HashRouter>
      <div className="flex h-screen w-screen flex-col overflow-hidden bg-background">
        {/* Window drag strip — full-width across the top, draggable via
            data-tauri-drag-region AND an explicit startDragging() handler
            so it works on every Tauri/macOS combination. */}
        <DragStrip />
        <div className="flex flex-1 overflow-hidden">
          <Sidebar onOpenSettings={() => setSettingsOpen(true)} />
          <main className="flex-1 overflow-y-auto">
            <Routes>
              <Route path="/" element={<Navigate to="/record" replace />} />
              <Route path="/record" element={<Record />} />
              <Route path="/library" element={<Library />} />
              <Route path="/editor" element={<Editor />} />
              <Route path="/tasks" element={<Tasks />} />
              <Route path="*" element={<Navigate to="/record" replace />} />
            </Routes>
          </main>
        </div>
        <SettingsModal open={settingsOpen} onOpenChange={setSettingsOpen} />
      </div>
    </HashRouter>
  );
}
