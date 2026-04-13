import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { WorkspaceLayout } from "./components/WorkspaceLayout";
import { ChatPage } from "./pages/ChatPage";
import { FilesPage } from "./pages/FilesPage";
import { TerminalPage } from "./pages/TerminalPage";
import { MemoryPage } from "./pages/MemoryPage";
import { SkillsPage } from "./pages/SkillsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { DashboardPage } from "./pages/DashboardPage";
import { InspectorPage } from "./pages/InspectorPage";
import { HUDPage } from "./pages/HUDPage";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<WorkspaceLayout />}>
          <Route index element={<Navigate to="/chat" replace />} />
          <Route path="chat" element={<ChatPage />} />
          <Route path="files" element={<FilesPage />} />
          <Route path="terminal" element={<TerminalPage />} />
          <Route path="memory" element={<MemoryPage />} />
          <Route path="skills" element={<SkillsPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="dashboard" element={<DashboardPage />} />
          <Route path="inspector" element={<InspectorPage />} />
          <Route path="hud" element={<HUDPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
