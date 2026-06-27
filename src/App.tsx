import { useMemo, useState } from "react";
import {
  BookOpen,
  Bot,
  Gauge,
  Library,
  MonitorCog,
  Settings,
} from "lucide-react";
import "./App.css";
import { AutomationView } from "./features/automation/AutomationView";
import { Dashboard } from "./features/dashboard/Dashboard";
import { LibraryView } from "./features/library/LibraryView";
import { ReaderView } from "./features/reader/ReaderView";
import { SettingsView } from "./features/settings/SettingsView";
import { approvedDefaultPaths, sampleFavorite } from "./test/fixtures";

type AppSection = "dashboard" | "library" | "automation" | "reader" | "settings";

const navigation: Array<{
  id: AppSection;
  label: string;
  icon: typeof Gauge;
}> = [
  { id: "dashboard", label: "仪表盘", icon: Gauge },
  { id: "library", label: "书库", icon: Library },
  { id: "automation", label: "自动化", icon: Bot },
  { id: "reader", label: "阅读器", icon: BookOpen },
  { id: "settings", label: "设置", icon: Settings },
];

function App() {
  const [activeSection, setActiveSection] = useState<AppSection>("dashboard");
  const favorites = useMemo(() => [sampleFavorite], []);

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="主导航">
        <div className="brand-block">
          <div className="brand-icon">
            <MonitorCog size={22} aria-hidden="true" />
          </div>
          <div>
            <strong>漫画控伴侣</strong>
            <span>Windows 本地工具</span>
          </div>
        </div>

        <nav className="nav-list">
          {navigation.map(({ id, label, icon: Icon }) => (
            <button
              className={activeSection === id ? "nav-item active" : "nav-item"}
              type="button"
              key={id}
              onClick={() => setActiveSection(id)}
            >
              <Icon size={18} aria-hidden="true" />
              <span>{label}</span>
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <span>状态</span>
          <strong>本地预览</strong>
        </div>
      </aside>

      <section className="content-shell">
        {activeSection === "dashboard" && (
          <Dashboard
            paths={approvedDefaultPaths}
            favorites={favorites}
            pendingTasks={3}
          />
        )}
        {activeSection === "library" && <LibraryView favorites={favorites} />}
        {activeSection === "automation" && <AutomationView />}
        {activeSection === "reader" && <ReaderView />}
        {activeSection === "settings" && <SettingsView paths={approvedDefaultPaths} />}
      </section>
    </main>
  );
}

export default App;
