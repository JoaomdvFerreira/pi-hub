import { RouterProvider } from "@/app/router";
import { AppShell } from "@/app/AppShell";
import { TerminalSessionsProvider } from "@/stores/useTerminalSessions";

function App() {
  return (
    <RouterProvider>
      <TerminalSessionsProvider>
        <AppShell />
      </TerminalSessionsProvider>
    </RouterProvider>
  );
}

export default App;
