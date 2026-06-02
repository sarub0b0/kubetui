# Config / Network label columns（Pod ミラー、共通スコープ）

- 日付: 2026-06-03
- ステータス: Proposed
- 対象範囲: Config / Network タブに column dialog と `label_columns` (config 由来) の対応を追加。Pod #993 と同型の構造を mirror。Pod/Node 並みに「config でラベル列を宣言し、表に出して、フィルタにも使える」UX を実現する。Config と Network は構造が酷似するため設計は本 spec に統合、実装は PR を別建てで進める（Config 先、Network 後）。
- 対象外: `column_presets` / `default_preset` / CLI `--config-columns` / `--network-columns` 引数の追加、kind 固有列 (例: Service の TYPE 列) の表示、per-kind sub-tab 化。これらは将来の独立 spec で扱う。

## 背景・動機

Pod #993 で `label_columns` が導入され、config の `theme.pod.label_columns` で宣言したラベルを Pod 表の列として表示・フィルタできるようになった。Config / Network タブは:

- PR #997 / #999 で column-aware filter までは Pod と同型化済み
- ただし `label_columns` 対応と column dialog は未着手

ユーザー視点では「Pod では `app.kubernetes.io/version` をラベル列にできるのに、Config/Network ではできない」という機能差が残っている。framework は十分成熟しているため、Pod と同じ pattern を mirror する形で揃えるのが自然。

### Config/Network 固有の事情（事前検討の確認）

設計の brainstorming で次の点を整理した:

1. **集約 view ゆえの kind 固有列の難しさ**: Service の TYPE/CLUSTER-IP, Ingress の HOSTS など各 kind 固有列を一律に並べると、非該当行で大量の空セルと横幅膨張が起きる。実用上ユーザーは「`kind:Service` で絞ってからその kind の列を見る」と動く。本 spec は**共通 (KIND/NAME/[DATA]/AGE) + label_columns** のみに絞り、kind 固有列は別 spec に分離する。
2. **preset の費用対効果**: Pod (9 builtin 列) と違い、Config (4) / Network (3) では preset の組み合わせ価値が小さく、label_columns 機能だけで実利は十分。`column_presets` / `default_preset` / CLI 引数は今回見送り（後から非破壊的に追加可能）。
3. **起動時の label 列表示**: preset なしの場合、`label_columns` に定義したラベルは**全て起動時に builtin の後ろに自動追加**する。dialog で個別 OFF 可能。「定義したら見える」直感的 UX。

## 現状確認

### Config タブ

- `src/features/config/kube/config.rs::fetch_configs` は `["KIND", "NAME", "DATA", "AGE"]` (multi-ns 時は `NAMESPACE` 先頭) の固定 4 列を構築。
- `KIND` セルは `ty.resource()` (`"ConfigMap"` / `"Secret"`) で固定。
- `src/features/config/filter/parser.rs::parse_config_filter` は `["NAME", "KIND", "DATA", "AGE"]` の固定 builtin を valid 列とする（PR #997）。registry を受け取らない。
- `src/features/config/view/widgets/config.rs` は `config_filter_applicator(tx)` を使用（PR #997）。column dialog 無し。
- `src/features/config.rs` モジュールに columns 型ファイル無し。
- config schema (`src/config/theme/config.rs`) は存在しない。`ThemeConfig` に `config` フィールド無し。

### Network タブ

- `src/features/network/kube/network.rs` は `NetworkTableRow {namespace, kind, name, age}` を集めて `["KIND", "NAME", "AGE"]` (multi-ns 時は `NAMESPACE` 先頭) の固定 3 列で出力。
- 6+ kind のサブ resource (Ingress/Service/Pod/NetworkPolicy/Gateway V1/V1Beta1/HTTPRoute V1/V1Beta1) を `fetch_resource` で並列取得 → merge。
- `parse_network_filter` は `["NAME", "KIND", "AGE"]` を valid 列とする（PR #999）。registry 受領なし。
- `src/features/network/view/widgets/network.rs` は `network_filter_applicator(tx)` を使用（PR #999）。column dialog 無し。
- `NetworkThemeConfig` は存在しない。

### Pod #993 で確立された共有資産

- `LabelColumnConfig` は `src/config/theme/label_column.rs` で共有（Pod / Node 両用、新 tab でも再利用）
- column-aware filter framework (PR #988-#992) は registry header を valid 列にできる
- メタデータ駆動の `CheckListItem` roundtrip (Pod の column dialog pattern) は再利用可能

## ゴール

1. config で `theme.config.label_columns: [{ name, label }, ...]` および `theme.network.label_columns` を宣言できる。
2. 宣言したラベル列が起動時から表に表示される（builtin default の後ろに自動追加）。
3. column dialog (`t` キー) で builtin 列と label 列を個別に ON/OFF できる。
4. **KIND と NAME は dialog で OFF にできない**（必須列）。
5. 列ダイアログから登録した列構成が runtime で controller 経由 poller に反映される（次の poll から該当列が描画）。
6. フィルタ式で label 列を絞り込みに使える（`app:nginx` 等、登録した header を valid 列として受領）。
7. `namespace:` Z モデル (PR #997 / #999) は破壊しない（registry より先に評価）。
8. Pod / Node と同型のコードパターンで、後から `column_presets` / CLI 引数 / kind 固有列を追加するときの干渉を最小化する。

## 非ゴール

- `column_presets` / `default_preset` の追加（将来 spec）
- CLI `--config-columns` / `--network-columns` の追加（将来 spec）
- kind 固有列の表示（Service の TYPE 等。集約 view の根本的な拡張は別 spec）
- per-kind sub-tab 化（Network タブの抜本的な UX 変更）
- label 値に対する独自の表示書式（色付け、表示幅指定など）

## 設計

### 1. 型 (Config / Network それぞれ新設)

Pod #993 の `PodColumnSpec` / `PodColumns` / `PodLabelColumn` と完全同型を Config と Network に新設する。

**Config**: `src/features/config/columns.rs`（新規）

```rust
#[derive(EnumIter, PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum ConfigColumn {
    Name,
    Kind,
    Data,
    Age,
}

impl ConfigColumn {
    pub const fn display(&self) -> &'static str;
    pub const fn normalize(&self) -> &'static str;
    pub fn normalize_column(s: &str) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigColumnSpec {
    Builtin(ConfigColumn),
    Label { key: String, header: String },
}

impl ConfigColumnSpec {
    pub fn header(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String, // = name.to_uppercase()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigColumns {
    columns: Vec<ConfigColumnSpec>,
}

impl ConfigColumns {
    pub fn new(columns: impl IntoIterator<Item = ConfigColumnSpec>) -> Self;
    pub fn from_builtins(columns: impl IntoIterator<Item = ConfigColumn>) -> Self;
    pub fn full() -> Self; // 全 builtin、label は app.rs 側で append
    pub fn specs(&self) -> &[ConfigColumnSpec];
    /// KIND と NAME を強制配置。canonical 順 (KIND 先頭, NAME 2 番目) を保つ:
    /// - KIND 不在なら index 0 に insert
    /// - NAME 不在なら KIND の直後 (index 1) に insert
    /// - 既存の順序は維持（label 列が間に挟まっていれば KIND/NAME の後ろに退かない）
    pub fn ensure_required(self) -> Self;
    pub fn dedup_columns(self) -> Self;
}

pub const DEFAULT_CONFIG_COLUMNS: &[ConfigColumn] = &[
    ConfigColumn::Kind,
    ConfigColumn::Name,
    ConfigColumn::Data,
    ConfigColumn::Age,
];
```

**Network**: `src/features/network/columns.rs`（新規）

同型。`NetworkColumn::{Name, Kind, Age}` の 3 種、`DEFAULT_NETWORK_COLUMNS` も同じ 3 種。

**Pod との違い**:

- `ensure_required`（KIND と NAME の 2 列）vs Pod の `ensure_name_column`（NAME のみ）
- builtin enum の種類数（4 / 3 vs Pod の 9）

### 2. Config schema

`LabelColumnConfig` は既存共有モジュール (`src/config/theme/label_column.rs`) を再利用。

**`src/config/theme/config.rs`（新規）**

```rust
use serde::{Deserialize, Serialize};

use super::LabelColumnConfig;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ConfigThemeConfig {
    /// Registry of label columns. 起動時に builtin の後ろへ全て追加される。
    /// dialog で個別 OFF 可。
    pub label_columns: Option<Vec<LabelColumnConfig>>,
}
```

**`src/config/theme/network.rs`（新規）** — 同型 `NetworkThemeConfig`。

**`src/config/theme.rs`**

```rust
mod config;
mod network;

pub use config::ConfigThemeConfig;
pub use network::NetworkThemeConfig;

pub struct ThemeConfig {
    // ... 既存
    pub config: ConfigThemeConfig,    // 新規
    pub network: NetworkThemeConfig,  // 新規
}
```

YAML 例:

```yaml
theme:
  config:
    label_columns:
      - { name: app, label: app.kubernetes.io/name }
      - { name: version, label: app.kubernetes.io/version }
  network:
    label_columns:
      - { name: app, label: app.kubernetes.io/name }
```

`Option<Vec<_>>` で `None` 既定なので既存 config はそのまま動く（後方互換）。

### 3. Message 拡張

既存の `ConfigMessage::Request(ConfigRequest)` は YAML 詳細データ取得用で名前を奪えないため、列構成更新は `ColumnsRequest` という別 variant にする（衝突回避）。

**`src/features/config/message.rs`**

```rust
pub enum ConfigMessage {
    Request(ConfigRequest),          // 既存: 詳細データ取得
    Response(ConfigResponse),        // 既存
    Filter(Option<String>),          // PR #997
    ColumnsRequest(ConfigColumns),   // 新規: dialog からの列構成更新
}
```

**`src/features/network/message.rs`** — 同型 `NetworkMessage::ColumnsRequest(NetworkColumns)`。

### 4. app.rs での registry 構築と配線

```rust
// 新規: registry builders
fn build_config_label_registry(
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Vec<ConfigLabelColumn>>;

fn build_network_label_registry(
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Vec<NetworkLabelColumn>>;

// 新規: 起動時の default 列組み立て（builtin default + 全 label を append）
fn build_default_config_columns(registry: &[ConfigLabelColumn]) -> ConfigColumns;
fn build_default_network_columns(registry: &[NetworkLabelColumn]) -> NetworkColumns;
```

各 registry builder は Pod #993 の `build_pod_label_registry` と同型のバリデーション:

- builtin と同名の `label_columns` エントリ → 起動時エラー
- label_columns 内で `header` の重複（normalize 後一致） → 起動時エラー

**startup フロー (app.rs main 抜粋)**:

```rust
let pod_label_registry = build_pod_label_registry(&config.theme.pod.label_columns)?;
let node_label_registry = build_node_label_registry(&config.theme.node.label_columns)?;
let config_label_registry = build_config_label_registry(&config.theme.config.label_columns)?;       // 新規
let network_label_registry = build_network_label_registry(&config.theme.network.label_columns)?;     // 新規

let default_config_columns = build_default_config_columns(&config_label_registry);
let default_network_columns = build_default_network_columns(&network_label_registry);

let shared_config_columns: SharedConfigColumns = Arc::new(RwLock::new(default_config_columns));      // 新規
let shared_network_columns: SharedNetworkColumns = Arc::new(RwLock::new(default_network_columns));   // 新規

// 既存
let shared_config_filter: SharedConfigFilter = Arc::new(RwLock::new(None));
let shared_network_filter: SharedNetworkFilter = Arc::new(RwLock::new(None));

ConfigPoller::new(
    tx.clone(),
    shared_target_namespaces.clone(),
    shared_config_columns.clone(),    // 新規
    shared_config_filter.clone(),
    client.clone(),
).spawn();

NetworkPoller::new(
    tx.clone(),
    shared_target_namespaces.clone(),
    shared_network_columns.clone(),   // 新規
    shared_network_filter.clone(),
    client.clone(),
    shared_api_resources.clone(),
).spawn();

// Tab 構築では registry を thread (widget と dialog で利用)
ConfigTab::new(..., config_label_registry, ...);
NetworkTab::new(..., network_label_registry, ...);
```

**Pod #993 との違い**:

- Pod は CLI / preset 経由 (`build_pod_columns`) で複雑な分岐を持つ。Config/Network は preset も CLI も無いので `build_default_<tab>_columns(registry)` の 1 関数で完結し、シンプル。

### 5. SharedConfigColumns / SharedNetworkColumns 型

`src/workers/kube/controller.rs` に追加:

```rust
pub type SharedConfigColumns = Arc<RwLock<ConfigColumns>>;
pub type SharedNetworkColumns = Arc<RwLock<NetworkColumns>>;
```

### 6. Controller の message handler

```rust
// 既存 Filter ハンドラの近くに追加
Kube::Config(ConfigMessage::ColumnsRequest(columns)) => {
    *shared_config_columns.write().await = columns;
}
Kube::Network(NetworkMessage::ColumnsRequest(columns)) => {
    *shared_network_columns.write().await = columns;
}
```

`EventControllerArgs` / `EventController` に `shared_config_columns` / `shared_network_columns` フィールドを追加し、`destructure` も更新（PR #993 と同様の流れ）。

### 7. Poller の spec 駆動化と label 値 render

**Config (`src/features/config/kube/config.rs`)**

`fetch_configs` を `shared_config_columns` から spec を読み、spec 順に cells を組み立てる形へ:

```rust
async fn fetch_configs(
    client: &KubeClient,
    namespaces: &[String],
    columns: &ConfigColumns,
    label_selector: Option<&str>,
) -> Result<KubeTable> {
    let specs = columns.specs();

    // header: NAMESPACE (multi-ns 時) + specs.iter().map(|s| s.header())
    let mut header: Vec<String> = specs.iter().map(|s| s.header()).collect();
    if namespaces.len() != 1 {
        header.insert(0, "NAMESPACE".to_string());
    }

    let jobs = try_join_all([
        fetch_configs_per_namespace(client, namespaces, Configs::ConfigMap, specs, label_selector),
        fetch_configs_per_namespace(client, namespaces, Configs::Secret,    specs, label_selector),
    ]).await?;

    Ok(KubeTable { header, rows: jobs.into_iter().flatten().collect() })
}
```

`fetch_configs_per_namespace` の closure は `build_config_row_cells(specs, kind_str, row, builtin_indexes)` helper に委譲して unit test 可能にする:

```rust
pub(crate) fn build_config_row_cells(
    specs: &[ConfigColumnSpec],
    kind: &str,
    row: &TableRow,
    builtin_indexes: &[usize], // Name/Data/Age の index（KIND は kind 引数から）
) -> Vec<String> {
    let mut builtin_iter = builtin_indexes.iter();
    specs.iter().map(|s| match s {
        ConfigColumnSpec::Builtin(ConfigColumn::Kind) => kind.to_string(),
        ConfigColumnSpec::Builtin(_) => {
            let i = builtin_iter.next().expect("builtin index available");
            row.cells[*i].to_string()
        }
        ConfigColumnSpec::Label { key, .. } => {
            row.object.as_ref()
                .and_then(|o| o.0.get("metadata"))
                .and_then(|m| m.get("labels"))
                .and_then(|l| l.get(key))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        }
    }).collect()
}
```

namespace の prepend は closure 内、`build_config_row_cells` の戻り値に対して既存と同じ処理。

**Network (`src/features/network/kube/network.rs`)**

同様。`NetworkTableRow` は中間表現として残しても、最終的に `to_kube_table_row` で spec 駆動の cells に変換する。helper:

```rust
pub(crate) fn build_network_row_cells(
    specs: &[NetworkColumnSpec],
    kind: &str,
    row: &TableRow,
    builtin_indexes: &[usize], // Name/Age の index
) -> Vec<String> { /* Pod の build_row_cells と同型 */ }
```

Network は sub-resource ごとに `fetch_table` を呼ぶ既存構造を維持し、最終的に行を spec 駆動で配列化する。

### 8. Filter parser の registry 対応

**Config (`src/features/config/filter/parser.rs`)**

```rust
pub fn parse_config_filter(
    input: &str,
    label_registry: &[ConfigLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let valid = valid_columns(label_registry);
    parse_table_filter(input, |column| {
        let normalized = normalize_column_name(column);
        if normalized == "namespace" {
            return Err("namespace is selected via the namespace selector, not the filter".into());
        }
        if valid.contains(&normalized) {
            Ok(())
        } else {
            Err(format!("unknown column '{}'", column))
        }
    })
}

fn valid_columns(label_registry: &[ConfigLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = ConfigColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .collect();
    for lc in label_registry {
        set.insert(normalize_column_name(&lc.header));
    }
    set
}
```

`config_filter_applicator` も `Vec<ConfigLabelColumn>` を受け取る:

```rust
pub fn config_filter_applicator(
    label_registry: Vec<ConfigLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str| parse_config_filter(input, &label_registry)).into();
    // ... 残りは既存
}
```

**Network 同型**: `parse_network_filter(input, &[NetworkLabelColumn])` / `network_filter_applicator(label_registry, tx)`。

### 9. Column dialog

新規ファイル: `src/features/config/view/widgets/config_columns_dialog.rs` / `src/features/network/view/widgets/network_columns_dialog.rs`。

Pod #993 の `pod_columns_dialog.rs` をテンプレートとして mirror:

- `candidate_specs(&label_registry)`: 全 builtin + 全 label
- `build_check_list_items(default_columns, &label_registry)`: 現在選択中→未選択候補の順
- `make_item(spec, checked)`: `required = matches!(spec, ConfigColumnSpec::Builtin(ConfigColumn::Kind) | ConfigColumnSpec::Builtin(ConfigColumn::Name))` （**Pod と違って KIND と NAME 両方が required**）
- `metadata_for(spec)`: `kind=builtin|label` + (`id` または `key + header`) で spec を roundtrip
- `spec_from_item(item)`: metadata から spec を復元
- `collect_columns(items)` → `ConfigColumns::new(specs).ensure_required().dedup_columns()`
- `on_change(tx)` → `ConfigMessage::ColumnsRequest(columns)` を送信

タイトル: `"Config Columns"` / `"Network Columns"`。

### 10. Widget 配線

**Config widget (`src/features/config/view/widgets/config.rs`)**

```rust
pub fn config_widget(
    tx: &Sender<Message>,
    label_registry: Vec<ConfigLabelColumn>,  // 新規
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    // ...
    Table::builder()
        // ...
        .filter_applicator(config_filter_applicator(label_registry, tx.clone()))
        .action('t', open_config_columns_dialog())  // 新規
        // ...
}
```

`open_config_columns_dialog`: `w.open_dialog(CONFIG_COLUMNS_DIALOG_ID)` を呼ぶだけの薄い関数。

**Network widget 同型**。

### 11. Tab 配線

**`src/features/config/view/tab.rs`**

```rust
pub struct ConfigTab {
    pub tab: Tab<'static>,
    pub config_columns_dialog: Widget<'static>,           // 新規
    pub config_filter_help_dialog: Widget<'static>,       // 既存
}

impl ConfigTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        default_columns: ConfigColumns,                   // 新規
        label_registry: Vec<ConfigLabelColumn>,           // 新規
        theme: WidgetThemeConfig,
    ) -> Self {
        let config_widget = config_widget(tx, label_registry.clone(), theme.clone());
        let raw_data_widget = raw_data_widget(clipboard, theme.clone());
        let config_columns_dialog = config_columns_dialog(tx, default_columns, label_registry, theme.clone());  // 新規
        let config_filter_help_dialog = config_filter_help_widget(theme);
        // ...
    }
}
```

**`Render` / `WindowInit`** に `default_config_columns` / `config_label_columns` / `default_network_columns` / `network_label_columns` フィールドを追加し、`ConfigTab::new` / `NetworkTab::new` に thread。

**`src/workers/render/window.rs`** で destructure し、`dialog_widgets` ベクタに `config_columns_dialog` / `network_columns_dialog` を push。

### 12. Component ID

`src/features/component_id.rs` に追加:

```rust
config_columns_dialog,
network_columns_dialog,
```

## リスク / 後方互換

- **schema 互換**: `theme.config` / `theme.network` セクションが既存 config に無くても `Default` で `label_columns: None` になるため、既存ユーザーは影響なし。
- **挙動互換**: `label_columns` を一切定義しないユーザーは現状と全く同じ列構成・挙動（`ensure_required` で KIND と NAME が保持される）。
- **column dialog の `required` 仕様**: KIND / NAME を OFF にできないため、ユーザーが「KIND を消したい」と思う極稀なケースで使えない。代替として `kind:Service` フィルタで実質的に絞れるためコストは小さい。
- **label header の重複検出**: PR #993 と同型の registry-build 時バリデーションを入れる。`app` と `APP` の同時定義は startup エラー。

## フォローアップ候補（未着手）

- Config/Network 用 `column_presets` / `default_preset` （要望が出たら追加）
- Config/Network 用 CLI 引数 (`--config-columns` / `--network-columns`)（同上）
- kind 固有列の表示（Service の TYPE/CLUSTER-IP, Ingress の HOSTS など）。集約 view の根本的な拡張で、別の brainstorming で (2)/(3)/(4) のいずれかを選択する
- Pod / Node の builtin enum と `<Tab>Column` が冗長なので、`<Tab>ColumnSpec` を生成するマクロ / 共通 trait の導入（4 タブが横並びになって初めて見える設計圧）

## 実装順序の方針

1. Config 用の型・config schema・registry・poller・dialog・filter wiring・テストを 1 PR
2. Config が main にマージされた後、Network 用に同型の変更を 1 PR
3. 各 PR の最終段で GKE 実機 smoke 確認（label 列が表示される、dialog 操作で ON/OFF できる、`label:` フィルタが効く、`<header>:value` フィルタが効く）

各 PR は plan を別途起こす（writing-plans skill で）。spec は本ドキュメント 1 件で両方をカバー。
