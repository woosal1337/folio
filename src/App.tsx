import * as React from "react";
import {
  HashRouter,
  Route,
  Routes,
  Navigate,
} from "react-router-dom";

import { Sidebar } from "@/components/sidebar";
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
        {/* Window drag strip. Spans the full width above the sidebar +
            content. macOS traffic lights live inside this region at the
            system level and stay clickable; everything else here is pure
            drag surface so the user can grab the window anywhere along
            the top. */}
        <div
          data-tauri-drag-region
          className="h-8 w-full shrink-0 bg-sidebar"
        />
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
