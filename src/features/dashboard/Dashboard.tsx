import {
  Activity,
  BookOpen,
  CheckCircle2,
  Database,
  DownloadCloud,
  FolderOpen,
  RefreshCw,
  Wrench,
} from "lucide-react";
import type { CompanionPaths, MangaConFavorite } from "../../lib/types";

interface DashboardProps {
  paths: CompanionPaths;
  favorites: MangaConFavorite[];
  pendingTasks: number;
  importMessage?: string;
  isImporting?: boolean;
  isUpdating?: boolean;
  isRepairing?: boolean;
  onImportFavorites?: () => void;
  onUpdateFavorites?: () => void;
  onRepairFailedTasks?: () => void;
}

export function Dashboard({
  paths,
  favorites,
  pendingTasks,
  importMessage,
  isImporting = false,
  isUpdating = false,
  isRepairing = false,
  onImportFavorites,
  onUpdateFavorites,
  onRepairFailedTasks,
}: DashboardProps) {
  const localCount = favorites.filter((item) => item.localPath).length;
  const pendingUpdates = favorites.filter(
    (item) => item.scanStatus === "pending" || item.scanStatus === "missing",
  ).length;
  const totalImages = favorites.reduce((sum, item) => sum + item.imageCount, 0);

  return (
    <section className="view dashboard-view" aria-labelledby="dashboard-title">
      <div className="view-header">
        <div>
          <p className="section-kicker">本地同步控制台</p>
          <h1 id="dashboard-title">漫画控伴侣</h1>
          <p className="view-subtitle">
            管理漫画控收藏、书架匹配和阅读准备任务。
          </p>
        </div>
        <div className="view-actions">
          <button
            className="primary-action"
            type="button"
            onClick={onUpdateFavorites}
            disabled={isUpdating}
          >
            <RefreshCw size={18} aria-hidden="true" />
            一键更新收藏
          </button>
          <button
            className="secondary-action"
            type="button"
            onClick={onRepairFailedTasks}
            disabled={isRepairing}
          >
            <Wrench size={18} aria-hidden="true" />
            修复失败图片
          </button>
          <button
            className="secondary-action"
            type="button"
            onClick={onImportFavorites}
            disabled={isImporting}
          >
            <Database size={18} aria-hidden="true" />
            导入漫画控收藏
          </button>
        </div>
      </div>

      <div className="metric-grid" aria-label="概览统计">
        <MetricCard
          icon={<BookOpen size={20} aria-hidden="true" />}
          label="收藏"
          value={favorites.length.toString()}
          detail="来自漫画控收藏导入"
          ariaLabel="收藏统计"
        />
        <MetricCard
          icon={<FolderOpen size={20} aria-hidden="true" />}
          label="本地漫画"
          value={localCount.toString()}
          detail="已匹配到书架目录"
        />
        <MetricCard
          icon={<DownloadCloud size={20} aria-hidden="true" />}
          label="待处理更新"
          value={pendingUpdates.toString()}
          detail="数据库队列"
        />
        <MetricCard
          icon={<Activity size={20} aria-hidden="true" />}
          label="任务"
          value={pendingTasks.toString()}
          detail="自动化队列中"
        />
      </div>

      <div className="two-column">
        <section className="panel" aria-labelledby="mangacon-status-title">
          <div className="panel-title-row">
            <CheckCircle2 size={20} aria-hidden="true" />
            <h2 id="mangacon-status-title">漫画控状态</h2>
          </div>
          <dl className="status-list">
            <div>
              <dt>程序路径</dt>
              <dd>{paths.mangaConExecutable}</dd>
            </div>
            <div>
              <dt>收藏快照</dt>
              <dd>{paths.mangaConFavoritesJson}</dd>
            </div>
            <div>
              <dt>漫画控数据库</dt>
              <dd>{paths.mangaConDatabase}</dd>
            </div>
            <div>
              <dt>书架根目录</dt>
              <dd>{paths.bookshelfRoot}</dd>
            </div>
            <div>
              <dt>本地数据库</dt>
              <dd>{paths.databasePath}</dd>
            </div>
          </dl>
        </section>

        <section className="panel" aria-labelledby="storage-title">
          <div className="panel-title-row">
            <Database size={20} aria-hidden="true" />
            <h2 id="storage-title">本地索引</h2>
          </div>
          <div className="index-summary">
            <strong>{totalImages}</strong>
            <span>已记录图片页</span>
          </div>
          <p className="muted">
            {importMessage ?? "导入漫画控收藏后，这里会显示真实书架匹配统计。"}
          </p>
        </section>
      </div>
    </section>
  );
}

interface MetricCardProps {
  icon: React.ReactNode;
  label: string;
  value: string;
  detail: string;
  ariaLabel?: string;
}

function MetricCard({ icon, label, value, detail, ariaLabel }: MetricCardProps) {
  return (
    <article className="metric-card" aria-label={ariaLabel}>
      <div className="metric-icon">{icon}</div>
      <div>
        <p>{label}</p>
        <strong>{value}</strong>
        <span>{detail}</span>
      </div>
    </article>
  );
}
