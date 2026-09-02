import { useCallback, useState } from "react";
import { ConfirmDialog } from "../components/ConfirmDialog";

export function useConfirm() {
  const [request, setRequest] = useState<{
    message: string;
    resolve: (value: boolean) => void;
  }>();

  const confirm = useCallback((message: string) => {
    return new Promise<boolean>((resolve) => {
      setRequest({ message, resolve });
    });
  }, []);

  function close(value: boolean) {
    request?.resolve(value);
    setRequest(undefined);
  }

  const dialog = request ? (
    <ConfirmDialog
      message={request.message}
      onConfirm={() => close(true)}
      onCancel={() => close(false)}
    />
  ) : null;

  return { confirm, dialog };
}
