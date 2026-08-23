import { useEffect, useState } from "react";
import {
  checkLocalInstallerUpdates,
  getAppVersion,
  openLocalInstaller,
} from "../lib/api";
import type { LocalInstallerPackage } from "../lib/types";

export function useAppUpdate() {
  const [appVersion, setAppVersion] = useState<string>();
  const [availableAppUpdate, setAvailableAppUpdate] =
    useState<LocalInstallerPackage>();
  const [isOpeningAppInstaller, setIsOpeningAppInstaller] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getAppVersion()
      .then((version) => {
        if (!cancelled) {
          setAppVersion(version);
        }
      })
      .catch(() => {
        // Version is decorative; ignore failures in non-Tauri shells.
      });

    checkLocalInstallerUpdates()
      .then((result) => {
        if (cancelled) {
          return;
        }
        setAvailableAppUpdate(
          result.hasUpdate && result.latest ? result.latest : undefined,
        );
      })
      .catch(() => {
        // Cloud update check is optional when offline.
      });

    return () => {
      cancelled = true;
    };
  }, []);

  async function installAppUpdate(): Promise<string> {
    if (!availableAppUpdate?.path) {
      return "";
    }
    setIsOpeningAppInstaller(true);
    try {
      await openLocalInstaller(availableAppUpdate.path);
      return `正在下载并安装 v${availableAppUpdate.version}，请按向导完成更新`;
    } finally {
      setIsOpeningAppInstaller(false);
    }
  }

  return {
    appVersion,
    availableAppUpdate,
    isOpeningAppInstaller,
    installAppUpdate,
  };
}
