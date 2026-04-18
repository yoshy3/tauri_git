# Tauri Git

[English version](./README.md)

Tauri / Svelte / Rust で構築した、ミニマルな Git GUI です。

## 概要

Tauri Git は、日常的な Git 操作をコンパクトに扱うことを重視したデスクトップクライアントです。

- ローカル Git リポジトリを開く
- ワーキングツリーの状態を確認する
- コミット履歴を参照する
- 変更をコミットする
- コミットダイアログから直前コミットを amend する
- stash を作成 / 適用 / pop する
- Fetch / Pull / Push を実行する
- amend 済みまたは履歴が分岐したブランチに対して、確認付きで force push する
- Pull / Push 対象コミット数をトップバーやサイドバーに表示する
- ブランチやリモートをツリー表示で参照する
- タグを参照し、履歴上の対象コミットへ移動する
- ブランチの checkout / 作成 / 削除を行う

## 技術スタック

- フロントエンド: Svelte + Vite
- バックエンド: Tauri 2 + Rust
- Git アクセス: `git2` と `git` コマンド実行
- i18n: `svelte-i18n`

## 現在の主な機能

- リポジトリ選択と、最後に開いたリポジトリの復元
- リポジトリのファイルや ref 変更に対する自動更新
- ワーキングツリーの変更一覧表示
- ローカル / リモート / タグ ref ラベル付きのコミット履歴表示
- コミット履歴の検索とフィルタ
- サイドバーでのブランチ検索とフィルタ
- サイドバーでタグを選択し、履歴上の対象コミットへスクロール
- WebView 上のタグ作成ダイアログ
- サイドバーメニューからのタグ削除
- タグ作成後に `origin` へ任意で push
- author / committer / refs / parents / changed files を含むコミット詳細表示
- コミット詳細から開ける左右比較 diff ダイアログ
- Changes Panel から開ける左右比較 diff ダイアログ
- Changes Panel からのコミット作成
- Changes Panel からの amend。直前コミットの summary / description を自動入力
- push 済みコミットを amend する際の警告表示
- stash の作成 / 適用 / pop
- サイドバーからの stash 選択と apply / pop
- `Fetch` / `Pull` / `Push` / `Refresh`
- amend 済みまたは履歴が分岐したブランチ向けの `--force-with-lease` push フロー
- `Pull` / `Push` ボタン上の入出力コミット数バッジ
- サイドバーのローカルブランチ名横への Pull / Push 対象コミット数表示
- `/` 区切りでネスト表示されるローカル / リモートブランチツリー
- ローカル / リモート ref からのブランチ checkout
- 作成後に自動 switch できるブランチ作成ダイアログ
- WebView 上のブランチ削除ダイアログ
- リモートブランチ削除
- ブランチ名入力付きの安全な削除確認
- 未マージのローカルブランチ向け force delete
- 英語 / 日本語 UI 切り替え
- ダーク / ライトテーマ切り替えと設定保存
- 左 / 中央 / 右ペイン幅のドラッグ調整
- ペイン幅の再起動後復元
- ウィンドウサイズの再起動後復元
- Wayland を除くウィンドウ位置の再起動後復元
- 起動時にちらつきを抑えたウィンドウ復元

## リリースビルド時の注意

### macOS

署名されていない、または notarization されていない macOS ビルドは、Gatekeeper により「アプリが壊れているため開けません」といった警告が出ることがあります。

配布物を信頼できる場合は、`Tauri Git.app` を `Applications` に移動してから quarantine 属性を外してください。

```bash
xattr -dr com.apple.quarantine "/Applications/Tauri Git.app"
```

その後、再度アプリを起動してください。

信頼できるリリースに対してのみ実行してください。なお起動できない場合は、ダウンロードしたファイル自体が壊れている可能性もあります。

### Windows

Windows では、ダウンロードしたリリースの起動時に Microsoft Defender SmartScreen が「Windows によって PC が保護されました」といった警告を表示する場合があります。

配布物を信頼できる場合は、次の手順で起動してください。

1. `詳細情報` をクリックする
2. 配布元とファイル名が、ダウンロードしたリリースと一致していることを確認する
3. `実行` または `Run anyway` を選ぶ

信頼できるリリースに対してのみ実行してください。想定外の入手元で警告が出た場合は、起動を中止してダウンロード元を確認してください。

## 開発

### 前提条件

- Node.js
- npm
- Rust toolchain
- 利用環境に応じた Tauri の前提ライブラリ
- `PATH` から利用できる Git

### 依存関係のインストール

```bash
npm install
```

### 開発モードで起動

```bash
npm run tauri dev
```

### フロントエンドをビルド

```bash
npm run build
```

### Windows の release ビルドをローカルで確認

Windows の release ビルドをローカルで確認したい場合は、次を実行してください。

```bash
npm run tauri build
```

その後、次の実行ファイルを起動します。

```powershell
.\src-tauri\target\release\tauri_git.exe
```

補足:

- release ビルドの確認には `npm run tauri build` を使ってください。ビルド済みのフロントエンド資産が Tauri アプリに正しく同梱されます。
- `cargo build --release` は Rust 側のコンパイル確認には便利ですが、パッケージ化されたアプリの動作確認には向いていません。
- `cargo build --release` で生成された exe を直接起動すると、`http://localhost:1420` を開こうとして `ERR_CONNECTION_REFUSED` になる場合があります。

## プロジェクト構成

```text
src/        Svelte フロントエンド
src-tauri/  Tauri + Rust バックエンド
```

## 補足

- 一部の Git 操作は、一般的なワークフローとの互換性のためシステムの `git` コマンドを使って実装しています。
- フル機能の Git GUI を目指すというより、日常的な作業に必要な操作へ絞った構成です。
- 左右比較ビューは、コミット詳細と Changes Panel の両方から利用できます。
- 通常のテキスト diff に対応しており、`git-crypt` 管理ファイル向けには compare view 用のフォールバックがあります。
- `git-crypt` 以外の独自 `diff` / `filter` / `textconv` 設定は、現在の compare view では未対応です。
- テーマ設定は次回起動時に自動復元されます。
- ペイン幅は次回起動時に自動復元されます。
- ウィンドウサイズは次回起動時に自動復元されます。Linux の Wayland セッションでは、ウィンドウ位置の復元に制限があります。

## ライセンス

このプロジェクトは MIT License のもとで公開されています。詳しくは [LICENSE](./LICENSE) を参照してください。
