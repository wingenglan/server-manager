import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { lazy, Suspense } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./app/AppShell";
import { OverviewPage } from "./features/overview/OverviewPage";
import { ServerLandingPage } from "./features/servers/ServerLandingPage";
import { SettingsPage } from "./features/settings/SettingsPage";
import "./App.css";

const FilesPage = lazy(() => import("./features/files/FilesPage").then((module) => ({ default: module.FilesPage })));
const TerminalPage = lazy(() => import("./features/terminal/TerminalPage").then((module) => ({ default: module.TerminalPage })));
const OperationsPage = lazy(() => import("./features/operations/OperationsPage").then((module) => ({ default: module.OperationsPage })));

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, refetchOnWindowFocus: false },
  },
});

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route element={<AppShell />}>
            <Route index element={<ServerLandingPage />} />
            <Route path="servers/:serverId" element={<OverviewPage />} />
            <Route path="servers/:serverId/files" element={<Suspense fallback={<div className="page-state">正在载入文件工作台…</div>}><FilesPage /></Suspense>} />
            <Route path="servers/:serverId/terminal" element={<Suspense fallback={<div className="page-state">正在载入终端…</div>}><TerminalPage /></Suspense>} />
            <Route path="servers/:serverId/operations" element={<Suspense fallback={<div className="page-state">正在载入运行现场…</div>}><OperationsPage /></Suspense>} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  );
}
