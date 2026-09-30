# README の画像（docs/images/hero.png）を描き出す。
#
# ui/ の画面に見本データ（mock.js）を差し込んだページを build/ に作り、
# Microsoft Edge のヘッドレスモードで hero.html を撮る。画面には何も表示しない。

$ErrorActionPreference = "Stop"
$src = $PSScriptRoot
$root = (Resolve-Path (Join-Path $src "..\..\..")).Path
$build = Join-Path $src "build"
New-Item -ItemType Directory -Force $build | Out-Null

$utf8 = New-Object System.Text.UTF8Encoding $false
foreach ($page in @("popover")) {
    $html = [System.IO.File]::ReadAllText((Join-Path $root "ui\$page.html"), $utf8)
    # 相対パスを ui/ に向け、画面のスクリプトより先に見本データを読み込む
    $inject = "<meta charset=`"utf-8`">`n<base href=`"../../../../ui/`">`n<script src=`"../docs/images/src/mock.js`"></script>"
    $html = $html.Replace("<meta charset=`"utf-8`">", $inject)
    [System.IO.File]::WriteAllText((Join-Path $build "$page.html"), $html, $utf8)
}

$edge = @(
    "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw "Microsoft Edge が見つかりません" }

# 普段使いの Edge のプロファイルに触れないよう、専用の一時フォルダを使う
$profileDir = Join-Path ([System.IO.Path]::GetTempPath()) "mira-readme-edge"
$out = Join-Path $root "docs\images\hero.png"
$url = "file:///" + ((Join-Path $src "hero.html") -replace "\\", "/")
$edgeArgs = @(
    "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
    "--user-data-dir=$profileDir", "--allow-file-access-from-files", "--hide-scrollbars",
    "--force-device-scale-factor=2", "--window-size=1280,640", "--virtual-time-budget=5000",
    "--screenshot=$out", $url
)
$p = Start-Process -FilePath $edge -ArgumentList $edgeArgs -Wait -PassThru -WindowStyle Hidden
if (-not (Test-Path $out)) { throw "画像を書けませんでした（終了コード $($p.ExitCode)）" }
Write-Output "書き出しました: $out"
