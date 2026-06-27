import { CheckCircle2, Clock3, ListChecks, PlayCircle, RefreshCw } from "lucide-react";

const timeline = [
  "监听漫画控收藏文件",
  "导入收藏快照",
  "扫描书架目录",
  "生成待阅读任务",
];

export function AutomationView() {
  return (
    <section className="view" aria-labelledby="automation-title">
      <div className="view-header compact">
        <div>
          <p className="section-kicker">自动化</p>
          <h1 id="automation-title">等待漫画控刷新收藏更新...</h1>
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
            <span>样本数 1</span>
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

      <div className="run-state">
        <RefreshCw size={18} aria-hidden="true" />
        <span>下一次检查：手动触发或漫画控文件更新</span>
        <button type="button">
          <PlayCircle size={16} aria-hidden="true" />
          立即检查
        </button>
      </div>
    </section>
  );
}
