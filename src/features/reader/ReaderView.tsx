import {
  Bookmark,
  Columns2,
  Expand,
  Maximize2,
  PanelTop,
  RotateCw,
  ZoomIn,
  ZoomOut,
} from "lucide-react";

const readerTools = [
  { label: "书签", icon: Bookmark },
  { label: "双页", icon: Columns2 },
  { label: "缩小", icon: ZoomOut },
  { label: "放大", icon: ZoomIn },
  { label: "全屏", icon: Maximize2 },
  { label: "方向", icon: RotateCw },
];

export function ReaderView() {
  return (
    <section className="view reader-view" aria-labelledby="reader-title">
      <div className="reader-toolbar" aria-label="阅读器工具栏">
        <div>
          <p className="section-kicker">阅读</p>
          <h1 id="reader-title">阅读器</h1>
        </div>
        <div className="tool-button-row">
          {readerTools.map(({ label, icon: Icon }) => (
            <button className="tool-button" type="button" key={label} aria-label={label}>
              <Icon size={17} aria-hidden="true" />
              <span>{label}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="reader-shell">
        <aside className="reader-side">
          <PanelTop size={18} aria-hidden="true" />
          <span>章节</span>
        </aside>
        <div className="reader-stage">
          <Expand size={32} aria-hidden="true" />
          <h2>选择一本本地漫画后开始阅读</h2>
          <p>这里会显示分页画面、缩放比例、方向和双页阅读状态。</p>
        </div>
      </div>
    </section>
  );
}
