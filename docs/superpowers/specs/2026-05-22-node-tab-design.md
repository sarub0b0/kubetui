# Node タブ設計

- 関連 Issue: [#920 feat: add Node tab for viewing node information](https://github.com/sarub0b0/kubetui/issues/920)
- ステータス: ドラフト（実装計画待ち）
- 作成日: 2026-05-22

## 概要

kubetui にトップレベルの「Node」タブを追加し、Kubernetes の Node を一覧・詳細表示できるようにする。一覧（テーブル）と詳細ペイン（Node の YAML ＋ そのノード上の関連 Pod）で構成し、ノード名や任意のラベルによる絞り込みフィルタを備える。

## 背景と動機

NVIDIA GPU ノードを運用するユーザーは、ノードのラベル（例: `nvidia.com/mig.config.state`）やステータスを頻繁に確認する必要がある。現状は `kubectl get nodes --show-labels` や `gron` など外部ツールに頼っている。これを kubetui 内で完結させる。

## ゴール / 非ゴール

### ゴール（v1）

- Node 一覧テーブル（既定列 `Name, Status, Roles, Age, Version`）
- 列のカスタマイズ（標準列＋wide 列、および任意ラベルの列表示）
- 詳細ペイン: Node の YAML 表示（`metadata.managedFields` を省略）＋自動更新
- 詳細ペイン末尾に関連 Pod（そのノード上の全 namespace の Pod）を追記
- フィルタ（`node:`/`!node:`/`label:`、Filter モードのみ）

### 非ゴール（将来検討）

- 詳細のサブタブ化（YAML ⇄ 関連 Pod を切替して各々を全画面表示する「案3」）
- 関連 Pod を独立した操作可能テーブルにする / 列ソート
- フィルタの Highlight モード（全件表示＋該当強調）
- **行の色分け（`node.highlights`）**。Pod のハイライトを「表示列依存（A）」から「データ駆動・表示非依存（B）」へ改修し、その仕組みを Node にも適用する**横断タスク**（[#969](https://github.com/sarub0b0/kubetui/issues/969)）として切り出す（本 Issue では対象外）。
- Node の操作系（drain / cordon / uncordon 等）

## UI / レイアウト（案1: コンパクト 2 ペイン）

tmux の小ペインやフローティングウィンドウでの利用を想定し、枠と余白を最小化して情報量を優先する。

- 縦積み 2 ペイン構成。
  - **上ペイン: Node 一覧（全幅）**。ノード名が長い（GKE 等のマネージドクラスタ）ケースと列数の多さを考慮し全幅を確保。
  - **下ペイン: 詳細（全幅）**。Node の YAML を表示し、その末尾に関連 Pod を追記する。
- **フィルタ入力は常駐させない**。一覧のタイトルに現在のフィルタ条件を表示し、`/` で 1 行入力を開いて編集する。
- 横スクロール/折り返しが多い YAML を全幅で扱えるため、案1 は折り返しを最小化できる。

```
┌ Node [2/8]  filter: node:worker  (/ to edit) ───────────┐
│ NAME                                STATUS ROLES AGE VER │
│ gke-prod-gpu-pool-e5f6a7b8-qw12     Ready  worker 10d ...│  ← 選択行
│ gke-prod-default-pool-a1b2c3d4-xz90 Ready  worker 10d ...│
└──────────────────────────────────────────────────────────┘
┌ Node YAML [1/512] ──────────────────────────────────────┐
│ apiVersion: v1                                           │
│ kind: Node                                               │
│ metadata:                                               │
│   name: gke-prod-gpu-pool-e5f6a7b8-qw12                 │
│   labels:                                               │
│     nvidia.com/mig.config.state: success               │
│ ...                                                     │
│                                                          │
│ relatedPods:                                            │
│ - namespace: gpu                                        │
│   name: gpu-train-0                                     │
│   status: Running                                       │
│ - namespace: gpu                                        │
│   name: dcgm-exporter-x9f2                              │
│   status: Running                                       │
└──────────────────────────────────────────────────────────┘
```

## アーキテクチャ

既存のフィーチャーモジュール規約（`src/features/{name}/`）に沿って、独立した `features/node/` モジュールを新設する（Pod タブを踏襲）。共有部品（`Table`/`Text` ウィジェット、`KubeClient`）を再利用する。

### モジュール構成

```
src/features/node/
├── message.rs              # NodeMessage(一覧) / NodeDetailMessage(YAML+関連Pod), From<…> for Message
├── node_columns.rs         # NodeColumn enum / NodeColumnSpec(Builtin|Label) / NodeLabelColumn / NodeColumns
├── filter.rs               # NodeFilter(AST) + 適用ロジック
├── filter/parser.rs        # nom ベースのフィルタパーサ
├── kube.rs                 # 再エクスポート
├── kube/
│   ├── node.rs             # NodePoller (InfiniteWorker): Node 一覧取得
│   └── detail.rs           # NodeDetailWorker: Node YAML + 関連 Pod 取得（自動更新）
└── view.rs / view/
    ├── tab.rs              # NodeTab（2 ペイン構成）
    └── widgets/
        ├── node.rs         # 一覧 Table ＋ on_select ＋ NodeFilter 適用 ＋ 列ダイアログ起動
        ├── detail.rs       # YAML＋関連Pod の Text ウィジェット（スクロール/検索）
        ├── node_filter.rs  # オンデマンドのフィルタ入力（/ で開く）
        ├── node_columns_dialog.rs  # 列選択ダイアログ（t キー）
        └── node_filter_help.rs     # フィルタ構文ヘルプダイアログ
```

### 既存ファイルの編集点（タッチポイント）

- `src/features.rs` — `pub mod node;` 追加
- `src/features/component_id.rs` — `NODE_TAB_ID` / `NODE_WIDGET_ID` / `NODE_DETAIL_WIDGET_ID` / `NODE_FILTER_WIDGET_ID` / ダイアログ ID 追加
- `src/workers/kube/message.rs` — `Kube` enum に `Node(NodeMessage)`（および詳細用メッセージ）を追加
- `src/workers/render/window.rs` — `NodeTab` を生成し、タブ配列の **Event の右隣**に挿入（後述）
- `src/workers/render/action.rs` — `update_contents()` に `NodeMessage::Poll` / 詳細レスポンスのハンドラを追加
- `src/workers/kube/controller.rs` — `NodePoller` の spawn、選択ノードの共有状態と `NodeDetailWorker` の起動
- `src/config/theme/node.rs`（新規） — `NodeThemeConfig`（`column_presets` / `default_preset` / `label_columns`）。`PodThemeConfig` の列プリセット部分に倣う（v1 では `highlights` は持たせない）
- `src/config/theme.rs` — `ThemeConfig` に `node: NodeThemeConfig` フィールドを追加（`theme.node` に配置。Pod の `theme.pod` と同様）

## データフロー

### 一覧（NodePoller）

- Node はクラスタスコープのため、namespace 選択は無視する。
- `request_table` を用いて `GET /api/v1/nodes?includeObject=Metadata`（`Node::url_path(&(), None)` ＋ クエリ）を **1 秒間隔**で取得する。`request_table` は `Accept: application/json;as=Table;...` を送るため Table 形式で返る。
- `includeObject=Metadata` により各 `TableRow.object`（`RawExtension`）に `PartialObjectMetadata`（`metadata.labels` 等）が載る。kubetui の `TableRow` は既に `object: Option<RawExtension>` を持つ（`src/kube/apis/v1_table.rs`）ため、追加のリクエストは不要。
- **標準列・wide 列**（`NAME, STATUS, ROLES, AGE, VERSION` ＋ `INTERNAL-IP, EXTERNAL-IP, OS-IMAGE, KERNEL-VERSION, CONTAINER-RUNTIME`）は `columnDefinitions` から名前で取得する。サーバ側プリンタが算出済みの値をそのまま使うため、Status/Roles/Age 等を自前で再実装しない。wide 列は `priority > 0` だが名前指定で取得可能。
- **ラベル列の値**は各 `TableRow.object`（`RawExtension`）→ `metadata.labels[key]` から取り出し、通常のセルとして行に格納する（`label_columns` で定義され、アクティブなプリセットが参照しているラベル列のみ）。該当ラベルを持たないノードは空セルにする（エラーにしない）。v1 では行の色分け（highlights）を持たないため、行に別途ラベル一式を保持する必要はない。
- 完成した `KubeTable` を `NodeMessage::Poll(Result<KubeTable>)` で送信し、render が一覧 Table ウィジェットを更新する。

### 詳細（NodeDetailWorker）

- 一覧で Node を選択すると、詳細ペインをクリアし、対象ノードを指定した `NodeDetailMessage::Request { name }` を送る。
- 詳細ワーカーは **request 駆動の `InfiniteWorker`** とし、対象ノードを取得して **3 秒間隔で自動更新**する。選択が変わると前のワーカーを abort し、新しい対象で起動し直す。この方式は Network の `NetworkDescriptionWorker`（`src/features/network/kube/description.rs`、`INTERVAL = 3`、選択ごとに対象を指定して起動する request 駆動の `InfiniteWorker`）に倣う。更新間隔 3 秒は `YamlWorker` / `NetworkDescriptionWorker` と揃える。
  1. Node オブジェクトを取得（`kube::Api::<Node>::get`）し、`metadata.managedFields` を除去してから YAML 化、行配列にする。
  2. 関連 Pod を `kube::Api::<Pod>::list(ListParams::default().fields("spec.nodeName=<node>"))`（**全 namespace**、クラスタスコープのリスト）で取得する。
  3. 関連 Pod を `relatedPods:` キー配下の YAML（各要素 `{namespace, name, status}`）に整形し、Node YAML の後に空行＋追記する（Network description の `relatedResources:` と同型・全体が単一の valid YAML ドキュメント）。`NodeDetailMessage::Response(Result<Vec<String>>)` として送信する。
- render が詳細 Text ウィジェット（スクロール・検索対応）を更新する。
- 関連 Pod が 0 件のときは関連 Pod セクションを出さない。

## 列設定

ビルトイン列の選択（プリセット）と、ラベル列の定義（レジストリ）を分ける。`column_presets` は **ビルトイン列名・ラベル列名のどちらも「文字列での参照」**になり、マップ混在やミニ構文を避けられる。

- 内部表現:
  - ビルトイン列 `NodeColumn`（`Copy` enum）:
    `Name, Status, Roles, Age, Version, InternalIP, ExternalIP, OSImage, KernelVersion, ContainerRuntime`
  - ランタイムの 1 列 = `NodeColumnSpec`（`Builtin(NodeColumn)` | `Label { key: String, header: String }`）。`NodeColumns = Vec<NodeColumnSpec>`。
  - ラベルレジストリ `NodeLabelColumn { name, key, header }`（`label_columns` を解決したもの）を app.rs で構築し、CLI・プリセット・ダイアログで共有する。
- 既定列: `Name, Status, Roles, Age, Version`。
- 設定は `NodeThemeConfig`（`theme.node`、`PodThemeConfig` に倣う）に以下を持たせる:
  - `label_columns: Vec<LabelColumnConfig>` — **ラベル列の定義**。各要素は `{ name, label }`（`name` = 参照名兼ヘッダ、`label` = Kubernetes ラベルキー）。
  - `column_presets: HashMap<preset_name, Vec<String>>` — 各プリセットは **ビルトイン列名 or 定義済みラベル列名** を文字列で並べる。
  - `default_preset: Option<String>`。
- 表示と参照:
  - プリセットからの参照は**大小無視でマッチ**（ビルトイン列名のパースと同じ正規化）。
  - ビルトイン列とラベル列は**任意順で混在（インターリーブ）**できる。プリセットは列名を並べた順がそのまま表示順になる。
  - 特殊名 `full` は**全ビルトイン列**に展開する（単独指定時のみ）。
  - ヘッダは**大文字化して表示**する（`name: zone` → 表示 `ZONE`。ビルトイン列の `NAME`/`STATUS` と揃える）。
  - 参照名と異なるヘッダにしたい場合に備え、将来 `LabelColumnConfig` に任意の `header` フィールドを足せる（v1 は `{ name, label }` の2項目）。
- CLI（`--node-columns` / `--node-columns-preset`）:
  - `--node-columns` は**列名の配列**（ビルトイン名・**ラベル列名**・`full` を指定可）。
  - 優先順位は `--node-columns` > `--node-columns-preset` > `default_preset`。
  - 環境変数（`KUBETUI_THEME__NODE__DEFAULT_PRESET` 等）でも切り替え可。
- **設定バリデーション（読み込み時にエラー）**:
  - ラベル列の `name` がビルトイン列名と衝突したらエラー。
  - プリセットがビルトイン名でも定義済みラベル列名でもない名前を参照したらエラー。
  - クラスタにそのラベルが存在しない場合は実行時にしか分からないため、エラーにせず空セル表示にする。
- `t` キーの列選択ダイアログ（CheckList）では**全ビルトイン列＋定義済みラベル列**をチェックボックスでトグルする（`Name` は常に含む）。チェック済みの既存列は順序を保ち、新規チェック列は末尾に追加する。
- ランタイムの列 = アクティブなプリセットが参照する「ビルトイン列＋ラベル列」。
- 設定例:

  ```yaml
  theme:
    node:
      label_columns:                          # 定義（名前 → ラベルキー）
        - name: zone
          label: failure-domain.beta.kubernetes.io/zone
        - name: region
          label: failure-domain.beta.kubernetes.io/region
        - name: mig
          label: nvidia.com/mig.config.state
      column_presets:                         # ビルトイン名 or ラベル列名を参照
        default:  [name, status, roles, age, version]
        topology: [name, status, region, zone]
        gpu:      [name, status, roles, mig]
      default_preset: default
  ```

## フィルタ

- 対象は **Node 一覧**。Pod のログフィルタのキーワード方式に倣うが、Node 一覧向けに簡素化する。
- キーワード:
  - `node:<regex>` — ノード名で絞り込み
  - `!node:<regex>` — ノード名で除外（複数指定可）
  - `label:<selector>` — ラベルセレクタ（例 `label:role=worker,zone=us-west`）
  - 複数語はスペース区切りで **AND** 結合。
- **モードは Filter モードのみ**（該当のみ表示）。Highlight モードは v1 では実装しない（理由は下記）。
- **適用は Enter キーで一括**（ライブフィルタは行わない）。`label:` がサーバリクエスト（`labelSelector`）を伴うため、`node:` だけライブにすると同一入力内で適用タイミングがばらつき分かりにくい。Pod のログクエリ（Enter で `exec_query`）と同じく、クエリ全体を Enter で適用する。
- 適用方法（Enter 時）:
  - パースしたフィルタを共有状態（`shared_node_filter`）に書き込む（Pod の `shared_pod_columns` と同じ流れ）。
  - `label:<selector>` → poller が Node 一覧リクエストの `labelSelector` に変換して**サーバ側**で絞り込む（内部データ参照。表示・非表示の列に依存しない）。
  - `node:`/`!node:` → poller が取得結果に**正規表現**を適用する。
  - 反映は次のポーリング（≤1 秒）。必要なら Enter 時に即時再取得をトリガしてもよい（任意）。
- Node 一覧では Table ウィジェット標準の部分一致フィルタではなく、この NodeFilter を採用する（`/` で開く入力が NodeFilter を駆動する）。
- UI: フィルタ入力は常駐させず、一覧タイトルに現在の条件を表示する。`/` で 1 行入力を開き、Enter で適用、Esc で取消、`?` でヘルプダイアログ。
- パースエラーは入力付近にインライン表示する（ログクエリと同様）。

### Highlight モードを v1 で見送る理由

- Highlight モード（全件表示＋該当強調）の本来の利点は「**前後の文脈を保ったまま該当を目立たせる**」ことで、これは時系列のある**ログ**で有効。Node 一覧は順序のない集合で行間に文脈がないため、利点が薄い。
- 「該当件数」は Filter モードでもタイトルの `[2/8]` で把握できる。
- 「異常なノードを目立たせたい」（`NotReady` 等）は本来フィルタの役割ではなく、**行の色分け（将来の横断的ハイライト機能。非ゴール参照）** で扱う。v1 では Status 列のテキストで判別する。
- 既存の Table ウィジェットは絞り込み（`filtered_items`）のみを持ち、Highlight モードには新規のウィジェット機能とモード切替 UI が必要でコストが高い。
- 将来必要になれば後から追加できる（「案3」と同様の段階的拡張）。

## 行の色分け（ハイライト）— v1 では対象外

行の色分けは v1 では実装しない。現状の `pod.highlights` は「**表示している列の値**」しか参照しない作り（例: Pod で STATUS 列を非表示にすると `pod.highlights` が効かない）であり、Node にデータ駆動（表示非依存）の色分けを入れると Pod と挙動が食い違って分かりにくくなる。世の中の一般的なメンタルモデル（表計算の条件付き書式・k9s 等の監視ツール）でも「**色の根拠はデータであり、表示列の有無で消えない**」のが期待値である。そこで、**Pod のハイライトをデータ駆動・表示非依存へ改修してから、同じ仕組みを Node にも適用する横断タスク**（[#969](https://github.com/sarub0b0/kubetui/issues/969)）として切り出す（非ゴール参照）。

## タブ配置とショートカット

- Node タブは利用頻度がそれほど高くないため、リソース系タブ（Pod / Config / Network / Event）の後ろ、**Event の右隣**に配置する。
- 結果のタブ順（番号キー）: `1 Pod` / `2 Config` / `3 Network` / `4 Event` / `5 Node` / `6 API` / `7 Yaml`。
- 番号キーはタブ配列の並び順で自動割当（`src/ui/window.rs`）。Node を挿入することで API / Yaml の番号が 1 つずつ繰り下がる。

## エラーハンドリング

- 既存の `NotifyError` / `ErrorSource`（`src/error.rs`）に倣い、`ErrorSource::Node` を追加する。
- poller / 詳細ワーカーのエラーは `Message::Error` で UI に通知し、ワーカーは継続する。
- 詳細取得失敗は詳細ペインに表示、フィルタ構文エラーは入力付近にインライン表示する（既存のエラー表示リファクタリングの方針に沿う）。
- 設定（`label_columns` の名前衝突、プリセットの未定義参照）は**読み込み時のエラー**として起動時に明示する（実行時のデータ依存エラーとは区別）。

## テスト

- インラインの `#[cfg(test)]` ユニットテストを中心にする。
- 対象:
  - NodeFilter パーサ（`node:` / `!node:` / `label:` / 複合）を `rstest` でパラメータ化
  - フィルタ適用ロジック（名前正規表現・ラベル一致・AND 結合）
  - ラベル列の抽出（`TableRow.object` → `metadata.labels`）
  - `managedFields` 除去
  - 関連 Pod の整形（0 件時にセクションを出さないこと含む）
  - ラベル列のヘッダ生成（参照名→大文字表示）と値の抽出（`metadata.labels` から、無い場合は空セル）
  - 設定バリデーション（ラベル列名とビルトイン列名の衝突、プリセットからの未定義参照をエラーにする）
- `mockall` で `KubeClientRequest` をモックし、`NodePoller` / `NodeDetailWorker` を検証（`mock_expect!` マクロを使用）。
- フィクスチャは `indoc`、差分表示は `pretty_assertions` を使用。

## 主要な設計判断の記録

- **取得方式は Table API（`includeObject=Metadata`）**。完全オブジェクト一覧＋自前算出案も検討したが、`kubectl get nodes -v 9` の確認で Table API レスポンスに `metadata.labels` が含まれることが分かり、kubetui の `TableRow.object` で受け取れる。サーバ算出値を使え、Pod タブと経路が一貫するため採用。
- **関連 Pod は全 namespace**（ノードのトラブルシュート用途を重視）。
- **フィルタは Filter モードのみ・Enter で一括適用**（ライブフィルタなし）。`label:` はサーバ側 `labelSelector`、`node:`/`!node:` はクライアント側（poller）正規表現で実現。これによりラベルを行ごとに view へ運ぶ必要がなくなり実装が軽い。
- **ラベル列は定義レジストリ `label_columns`（`{ name, label }`）＋プリセットからの名前参照**で扱う。`column_presets` はビルトイン名・ラベル列名ともに「文字列の参照」だけになり、混在やミニ構文を避けられる。`--custom-columns`/jsonpath はサーバ Table を捨ててフル取得が必要なため v1 では採らない。
- **設定の整合性は読み込み時にエラー検証**する（ラベル列名のビルトイン衝突、プリセットの未定義参照）。クラスタにラベルが存在しないケースは実行時にしか分からないためエラーにせず空表示。
- **行の色分け（ハイライト）は v1 では見送り**。現状の Pod ハイライトが表示列依存である課題が判明したため、Pod を「データ駆動・表示非依存」へ改修してから Node に適用する横断タスク（[#969](https://github.com/sarub0b0/kubetui/issues/969)）に分離（タブ間の挙動一貫性とメンタルモデル整合を優先）。
- **レイアウトは案1（コンパクト 2 ペイン）から開始**。将来「案3（詳細サブタブ化）」へ発展可能。関連 Pod は個人的な利便性での追加であり、まずは YAML 末尾追記の軽量実装とする。
