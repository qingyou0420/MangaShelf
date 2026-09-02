import { MoreHorizontal } from "lucide-react";
import { useEffect, useRef, useState } from "react";

export interface OverflowMenuItem {
  id: string;
  label: string;
  hidden?: boolean;
  onSelect: () => void;
}

interface OverflowMenuProps {
  label?: string;
  items: OverflowMenuItem[];
}

export function OverflowMenu({
  label = "更多操作",
  items,
}: OverflowMenuProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const visible = items.filter((item) => !item.hidden);

  useEffect(() => {
    function onPointer(event: MouseEvent) {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
      }
    }
    window.addEventListener("mousedown", onPointer);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onPointer);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  if (visible.length === 0) {
    return null;
  }

  return (
    <div className="overflow-menu" ref={rootRef}>
      <button
        type="button"
        className={open ? "icon-action active" : "icon-action"}
        aria-label={label}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        <MoreHorizontal size={16} />
      </button>
      {open && (
        <div className="overflow-menu-panel" role="menu">
          {visible.map((item) => (
            <button
              key={item.id}
              type="button"
              role="menuitem"
              className="overflow-menu-item"
              onClick={() => {
                setOpen(false);
                item.onSelect();
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
