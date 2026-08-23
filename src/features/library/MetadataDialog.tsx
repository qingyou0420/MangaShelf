import { useEffect, useState } from "react";
import type { LibraryComic } from "../../lib/types";

interface MetadataDialogProps {
  comic: LibraryComic;
  onSave: (draft: { name: string; author: string; tags: string }) => void;
  onClose: () => void;
}

export function MetadataDialog({ comic, onSave, onClose }: MetadataDialogProps) {
  const [draftName, setDraftName] = useState(comic.name);
  const [draftAuthor, setDraftAuthor] = useState(comic.author ?? "");
  const [draftTags, setDraftTags] = useState((comic.tags ?? []).join("，"));

  useEffect(() => {
    setDraftName(comic.name);
    setDraftAuthor(comic.author ?? "");
    setDraftTags((comic.tags ?? []).join("，"));
  }, [comic.id, comic.name, comic.author, comic.tags]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="modal-backdrop"
      role="presentation"
      onClick={onClose}
    >
      <form
        className="modal-dialog"
        role="dialog"
        aria-labelledby="edit-comic-title"
        onClick={(event) => event.stopPropagation()}
        onSubmit={(event) => {
          event.preventDefault();
          onSave({
            name: draftName,
            author: draftAuthor,
            tags: draftTags,
          });
        }}
      >
        <div className="modal-header">
          <h2 id="edit-comic-title">编辑元数据</h2>
        </div>
        <label className="settings-kv">
          <span>标题</span>
          <input
            value={draftName}
            onChange={(event) => setDraftName(event.target.value)}
            aria-label="标题"
          />
        </label>
        <label className="settings-kv">
          <span>作者</span>
          <input
            value={draftAuthor}
            onChange={(event) => setDraftAuthor(event.target.value)}
            aria-label="作者"
          />
        </label>
        <label className="settings-kv">
          <span>标签</span>
          <input
            value={draftTags}
            onChange={(event) => setDraftTags(event.target.value)}
            aria-label="标签"
            placeholder="用逗号分隔"
          />
        </label>
        <div className="modal-actions">
          <button type="button" className="secondary-action" onClick={onClose}>
            取消
          </button>
          <button type="submit" className="primary-action">
            保存
          </button>
        </div>
      </form>
    </div>
  );
}
