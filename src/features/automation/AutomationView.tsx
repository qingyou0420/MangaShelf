import {
  CheckCircle2,
  Clock3,
  ListChecks,
  MonitorDot,
  PanelTopOpen,
  PlayCircle,
  RefreshCw,
  ScanSearch,
  Search,
} from "lucide-react";
import { useState } from "react";
import {
  findMangaConWindows,
  getAutomationStatus,
  launchMangaCon,
  openMangaConFavorites,
  scanMangaConBadges,
} from "../../lib/api";
import type {
  AutomationRunStatus,
  CompanionPaths,
  LaunchMangaConResult,
  MangaConBadgeScanResult,
  MangaConWindow,
  OpenFavoritesResult,
} from "../../lib/types";
import { approvedDefaultPaths } from "../../lib/defaults";

const timeline = [
  "监听漫画控收藏文件",
  "导入收藏快照",
  "扫描书架目录",
  "生成待阅读任务",
];

const fixtureStatus: AutomationRunStatus = {
  state: "waiting_refresh",
  message: "等待漫画控刷新收藏更新...",
  detectedBadges: 0,
  stableSamples: 1,
};

interface AutomationViewProps {
  status?: AutomationRunStatus;
  paths?: CompanionPaths;
  service?: AutomationService;
}

export interface AutomationService {
  findWindows: () => Promise<MangaConWindow[]>;
  launch: (executablePath: string) => Promise<LaunchMangaConResult>;
  getStatus: () => Promise<AutomationRunStatus>;
  scanBadges: () => Promise<MangaConBadgeScanResult>;
  openFavorites: () => Promise<OpenFavoritesResult>;
}

const defaultAutomationService: AutomationService = {
  findWindows: findMangaConWindows,
  launch: (executablePath) => launchMangaCon({ executablePath }),
  getStatus: getAutomationStatus,
  scanBadges: scanMangaConBadges,
  openFavorites: openMangaConFavorites,
};

export function AutomationView({
  status = fixtureStatus,
  paths = approvedDefaultPaths,
  service = defaultAutomationService,
}: AutomationViewProps) {
  const [currentStatus, setCurrentStatus] = useState(status);
  const [windows, setWindows] = useState<MangaConWindow[]>([]);
  const [launchResult, setLaunchResult] = useState<LaunchMangaConResult>();
  const [badgeScan, setBadgeScan] = useState<MangaConBadgeScanResult>();
  const [openResult, setOpenResult] = useState<OpenFavoritesResult>();
  const [message, setMessage] = useState("尚未联动漫画控");
  const [busyAction, setBusyAction] = useState<string>();
  const [error, setError] = useState<string>();

  async function runAction(actionName: string, action: () => Promise<void>) {
    setBusyAction(actionName);
    setError(undefined);
    try {
      await action();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusyAction(undefined);
    }
  }

  function handleFindWindows() {
    void runAction("find", async () => {
      const foundWindows = await service.findWindows();
      setWindows(foundWindows);
      setMessage(
        foundWindows.length > 0
          ? `已发现 ${foundWindows.length} 个窗口`
          : "没有发现漫画控窗口",
      );
    });
  }

  function handleLaunch() {
    void runAction("launch", async () => {
      const result = await service.launch(paths.mangaConExecutable);
      setLaunchResult(result);
      setMessage("启动请求已发送，正在查找窗口");
      setWindows(await service.findWindows());
    });
  }

  function handleRefreshStatus() {
    void runAction("status", async () => {
      setCurrentStatus(await service.getStatus());
      setMessage("自动化状态已刷新");
    });
  }

  function handleScanBadges() {
    void runAction("scan", async () => {
      const result = await service.scanBadges();
      setBadgeScan(result);
      setWindows([result.window]);
      setMessage("截图识别完成");
    });
  }

  function handleOpenFavorites() {
    void runAction("favorites", async () => {
      const result = await service.openFavorites();
      setOpenResult(result);
      setBadgeScan(result);
      setWindows([result.window]);
      setMessage("收藏夹已打开");
    });
  }

  return (
    <section className="view" aria-labelledby="automation-title">
      <div className="view-header compact">
        <div>
          <p className="section-kicker">自动化</p>
          <h1 id="automation-title">{currentStatus.message}</h1>
          <p className="view-subtitle">
            当前处于观察状态，刷新后会触发导入、匹配和任务整理。
          </p>
        </div>
        <button className="secondary-action" type="button">
          <ListChecks size={18} aria-hidden="true" />
          查看任务队列
        </button>
      </div>

      <div className="automation-grid">
        <section className="panel" aria-labelledby="sample-title">
          <div className="panel-title-row">
            <CheckCircle2 size={20} aria-hidden="true" />
            <h2 id="sample-title">稳定样本</h2>
          </div>
          <p className="muted">
            使用已知收藏样本验证导入链路，避免在 UI 壳阶段依赖真实文件变动。
          </p>
          <div className="sample-strip">
            <span>红点 {currentStatus.detectedBadges}</span>
            <span>稳定样本 {currentStatus.stableSamples}</span>
            <span>匹配数 0</span>
            <span>错误 0</span>
          </div>
        </section>

        <section className="panel" aria-labelledby="timeline-title">
          <div className="panel-title-row">
            <Clock3 size={20} aria-hidden="true" />
            <h2 id="timeline-title">流程时间线</h2>
          </div>
          <ol className="timeline">
            {timeline.map((item, index) => (
              <li key={item}>
                <span>{index + 1}</span>
                <p>{item}</p>
              </li>
            ))}
          </ol>
        </section>
      </div>

      <section className="panel mangacon-link-panel" aria-labelledby="mangacon-link-title">
        <div className="panel-title-row">
          <MonitorDot size={20} aria-hidden="true" />
          <h2 id="mangacon-link-title">漫画控联动</h2>
        </div>
        <div className="link-toolbar">
          <button
            className="secondary-action"
            type="button"
            onClick={handleFindWindows}
            disabled={busyAction === "find"}
          >
            <Search size={16} aria-hidden="true" />
            查找漫画控窗口
          </button>
          <button
            className="primary-action"
            type="button"
            onClick={handleLaunch}
            disabled={busyAction === "launch"}
          >
            <PlayCircle size={16} aria-hidden="true" />
            启动漫画控
          </button>
          <button
            className="secondary-action"
            type="button"
            onClick={handleRefreshStatus}
            disabled={busyAction === "status"}
          >
            <RefreshCw size={16} aria-hidden="true" />
            刷新状态
          </button>
          <button
            className="secondary-action"
            type="button"
            onClick={handleScanBadges}
            disabled={busyAction === "scan"}
          >
            <ScanSearch size={16} aria-hidden="true" />
            识别红点
          </button>
          <button
            className="secondary-action"
            type="button"
            onClick={handleOpenFavorites}
            disabled={busyAction === "favorites"}
          >
            <PanelTopOpen size={16} aria-hidden="true" />
            打开收藏夹
          </button>
        </div>
        <dl className="link-state">
          <div>
            <dt>程序路径</dt>
            <dd>{paths.mangaConExecutable}</dd>
          </div>
          <div>
            <dt>窗口状态</dt>
            <dd>{message}</dd>
          </div>
          {launchResult && (
            <div>
              <dt>最近启动</dt>
              <dd>已启动 PID {launchResult.pid}</dd>
            </div>
          )}
          {badgeScan && (
            <>
              <div>
                <dt>截图尺寸</dt>
                <dd>截图 {badgeScan.width}x{badgeScan.height}</dd>
              </div>
              <div>
                <dt>红点结果</dt>
                <dd>识别红点 {badgeScan.badges.length}</dd>
              </div>
            </>
          )}
          {openResult && (
            <div>
              <dt>最近点击</dt>
              <dd>点击 {openResult.clicked.x},{openResult.clicked.y}</dd>
            </div>
          )}
          {error && (
            <div>
              <dt>错误</dt>
              <dd>{error}</dd>
            </div>
          )}
        </dl>
        <div className="window-list" aria-label="漫画控窗口列表">
          {windows.length === 0 ? (
            <span>暂无窗口记录</span>
          ) : (
            windows.map((window) => (
              <span key={window.hwnd}>
                {window.title}
              </span>
            ))
          )}
        </div>
      </section>

      <div className="run-state">
        <RefreshCw size={18} aria-hidden="true" />
        <span>下一次检查：手动触发或漫画控文件更新</span>
        <button type="button" onClick={handleRefreshStatus}>
          <PlayCircle size={16} aria-hidden="true" />
          立即检查
        </button>
      </div>
    </section>
  );
}
