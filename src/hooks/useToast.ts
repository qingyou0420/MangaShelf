import { useCallback, useEffect, useRef, useState } from "react";

export function useToast(durationMs = 4_500) {
  const [toastMessage, setToastMessage] = useState<string>();
  const toastTimerRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    return () => {
      if (toastTimerRef.current !== undefined) {
        window.clearTimeout(toastTimerRef.current);
      }
    };
  }, []);

  const showToast = useCallback(
    (message: string) => {
      setToastMessage(message);
      if (toastTimerRef.current !== undefined) {
        window.clearTimeout(toastTimerRef.current);
      }
      toastTimerRef.current = window.setTimeout(() => {
        toastTimerRef.current = undefined;
        setToastMessage(undefined);
      }, durationMs);
    },
    [durationMs],
  );

  return { toastMessage, showToast };
}
