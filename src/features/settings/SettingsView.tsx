import { FolderCog, HardDrive, Save } from "lucide-react";
import type { CompanionPaths } from "../../lib/types";

interface SettingsViewProps {
  paths: CompanionPaths;
}

export function SettingsView({ paths }: SettingsViewProps) {
  return (
    <section className="view" aria-labelledby="settings-title">
      <div className="view-header compact">
        <div>
          <p className="section-kicker">设置</p>
          <h1 id="settings-title">路径与默认项</h1>
          <p className="view-subtitle">先展示批准的默认路径，后续再接入可编辑配置。</p>
        </div>
        <button className="secondary-action" type="button">
          <Save size={18} aria-hidden="true" />
          保存
        </button>
      </div>

      <section className="panel settings-panel" aria-label="默认路径">
        <PathRow
          icon={<FolderCog size={20} aria-hidden="true" />}
          label="漫画控程序"
          value={paths.mangaConExecutable}
        />
        <PathRow
          icon={<HardDrive size={20} aria-hidden="true" />}
          label="收藏快照"
          value={paths.mangaConFavoritesJson}
        />
        <PathRow
          icon={<FolderCog size={20} aria-hidden="true" />}
          label="书架目录"
          value={paths.bookshelfRoot}
        />
        <PathRow
          icon={<HardDrive size={20} aria-hidden="true" />}
          label="本地数据库"
          value={paths.databasePath}
        />
      </section>
    </section>
  );
}

interface PathRowProps {
  icon: React.ReactNode;
  label: string;
  value: string;
}

function PathRow({ icon, label, value }: PathRowProps) {
  return (
    <div className="path-row">
      <div className="path-icon">{icon}</div>
      <div>
        <span>{label}</span>
        <code>{value}</code>
      </div>
    </div>
  );
}
