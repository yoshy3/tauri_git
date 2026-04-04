# Tauri Git

[English version](./README.md)

Tauri / Svelte / Rust で構成した、ミニマルな Git GUI です。

## 概要

Tauri Git は、日常的な Git 操作をコンパクトに扱うためのデスクトップアプリです。

- ローカル Git リポジトリを開く
- 作業ツリーの状態を確認する
- コミット履歴を閲覧する
- コミットを作成する
- stash を作成・適用・pop する
- Fetch / Pull / Push を実行する
- ブランチとリモートをツリー表示で確認する
- ブランチの checkout / 作成 / 削除を行う

## 技術スタック

- フロントエンド: Svelte + Vite
- バックエンド: Tauri 2 + Rust
- Git 操作: `git2` と `git` コマンド実行
- 多言語対応: `svelte-i18n`

## 現在の主な機能

- リポジトリ選択と前回開いたリポジトリの復元
- 作業ツリーの変更一覧表示
- ローカル / リモート ref ラベル付きのコミット履歴表示
- Changes Panel からのコミット作成
- stash の作成 / 適用 / pop
- `Fetch` / `Pull` / `Push` / `Refresh`
- `/` 区切りを階層として扱うローカル / リモートブランチツリー
- ローカル / リモートブランチからの checkout
- 作成後の自動切り替えに対応したブランチ作成ダイアログ
- WebView ベースのブランチ削除ダイアログ
- リモートブランチ削除対応
- ブランチ名入力必須の安全な削除確認
- 未マージのローカルブランチ向け強制削除オプション
- 英語 / 日本語 UI 切り替え

## 開発

### 前提

- Node.js
- npm
- Rust toolchain
- 利用環境に応じた Tauri の前提ライブラリ
- `PATH` から参照できる Git

### 依存関係のインストール

```bash
npm install
```

### 開発起動

```bash
npm run tauri dev
```

### フロントエンドのビルド

```bash
npm run build
```

## ディレクトリ構成

```text
src/        Svelte フロントエンド
src-tauri/  Tauri + Rust バックエンド
```

## 補足

- 一部の Git 操作は、一般的な Git ワークフローとの互換性を優先してシステムの `git` コマンドを利用しています。
- 現在は高機能な Git GUI の完全再現ではなく、日常用途に必要な最小構成を重視しています。

## ライセンス

このプロジェクトは MIT License のもとで公開されています。詳細は [LICENSE](./LICENSE) を参照してください。
