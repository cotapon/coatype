import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button, Spinner } from "@heroui/react";
import { checkForUpdate, downloadAndInstallUpdate } from "./updater";
import type { Update } from "./updater";
import logoUrl from "./assets/logo.png";

/**
 * アップデートダイアログ。
 *
 * - マウント時に checkForUpdate() を実行し、更新があればウィンドウを表示する。
 * - 更新がない場合・エラーの場合はウィンドウをそのまま close する(ユーザーには見えない)。
 * - ウィンドウは decorations:false / transparent なので、角丸カードを自前で描画する。
 * - OS テーマ追従は main.tsx の applySystemTheme で適用済み。
 */
export function UpdateDialog() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const found = await checkForUpdate();
        if (found) {
          setUpdate(found);
          const win = getCurrentWindow();
          // macOS ネイティブウィンドウシャドウを無効化(透過ウィンドウで楕円が出るため)
          await win.setShadow(false);
          await win.center();
          await win.show();
          await win.setFocus();
        } else {
          await getCurrentWindow().close();
        }
      } catch {
        // ネットワーク不通・署名エラー等はサイレントに閉じる
        await getCurrentWindow().close();
      }
    })();
  }, []);

  const handleInstall = async () => {
    if (!update) return;
    setInstalling(true);
    try {
      await downloadAndInstallUpdate(update);
      // downloadAndInstallUpdate 内で relaunch() が呼ばれるため、ここには到達しない。
    } catch {
      setInstalling(false);
    }
  };

  const handleLater = async () => {
    await getCurrentWindow().close();
  };

  // update が確定するまでは何も描画しない(close() が呼ばれるのを待つ)
  if (!update) return null;

  return (
    <div className="flex h-screen w-screen select-none items-center justify-center">
      <div className="flex w-[320px] flex-col items-center gap-5 rounded-2xl bg-background px-6 py-8">
        {/* ロゴ */}
        <div className="flex size-16 shrink-0 items-center justify-center rounded-xl border border-border bg-white p-1.5">
          <img src={logoUrl} alt="CoAType" className="size-full object-contain" />
        </div>

        {/* テキスト */}
        <div className="flex flex-col items-center gap-2 text-center">
          <h2 className="text-base font-semibold text-foreground">
            アップデートが利用可能です
          </h2>
          <p className="text-sm text-muted">
            CoAType バージョン {update.version} が利用可能です。
          </p>
          {update.body && (
            <p className="whitespace-pre-wrap text-xs text-muted">{update.body}</p>
          )}
          <p className="text-sm text-muted">今すぐ更新しますか？</p>
        </div>

        {/* ボタン */}
        <div className="flex w-full gap-3">
          <Button
            variant="secondary"
            className="flex-1"
            onPress={handleLater}
            isDisabled={installing}
          >
            後で
          </Button>
          <Button
            variant="primary"
            className="flex-1"
            onPress={handleInstall}
            isDisabled={installing}
          >
            {installing ? (
              <>
                <Spinner className="size-4" />
                更新中…
              </>
            ) : (
              "今すぐ更新"
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}
