# Changelog

## [v0.6.0](https://github.com/sarub0b0/kubetui/tree/v0.6.0) (2021-11-07)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.5.1...v0.6.0)

**Closed issues:**

- Podのログ取得で対象のコンテナが切り替わるときにログが取得できない問題がある [\#113](https://github.com/sarub0b0/kubetui/issues/113)
- context切り替え後に選択中のAPIsがリストアされない問題を修正 [\#109](https://github.com/sarub0b0/kubetui/issues/109)
- LogsとRaw Dataのタイトルに選択しているアイテム名を付け足す [\#101](https://github.com/sarub0b0/kubetui/issues/101)
- widgetのタイトル更新APIを実装する [\#95](https://github.com/sarub0b0/kubetui/issues/95)
- 選択したリソース情報をyamlで表示（pod, apis） [\#76](https://github.com/sarub0b0/kubetui/issues/76)

**Merged pull requests:**

- fix\(deps\): update rust crate nom to 7.1.0 [\#114](https://github.com/sarub0b0/kubetui/pull/114) ([sarub0b0](https://github.com/sarub0b0))
- yaml表示機能 [\#110](https://github.com/sarub0b0/kubetui/pull/110) ([sarub0b0](https://github.com/sarub0b0))
- chore\(deps\): update all dependencies [\#108](https://github.com/sarub0b0/kubetui/pull/108) ([sarub0b0](https://github.com/sarub0b0))
- chore\(deps\): update rust crate proc-macro2 to 1.0.31 [\#107](https://github.com/sarub0b0/kubetui/pull/107) ([sarub0b0](https://github.com/sarub0b0))
- fix\(deps\): update rust crate crossterm to v0.22.1 [\#105](https://github.com/sarub0b0/kubetui/pull/105) ([sarub0b0](https://github.com/sarub0b0))
- chore\(deps\): update rust crate proc-macro2 to 1.0.30 [\#103](https://github.com/sarub0b0/kubetui/pull/103) ([sarub0b0](https://github.com/sarub0b0))
- fix\(deps\): update kube-rs \(kube, kube-runtime, k8s-openapi\) to 0.61 [\#102](https://github.com/sarub0b0/kubetui/pull/102) ([sarub0b0](https://github.com/sarub0b0))
- widgetのタイトル更新関数を実装 [\#100](https://github.com/sarub0b0/kubetui/pull/100) ([sarub0b0](https://github.com/sarub0b0))
- fix\(deps\): update rust crate regex to 1.5.4 [\#99](https://github.com/sarub0b0/kubetui/pull/99) ([sarub0b0](https://github.com/sarub0b0))
- fix\(deps\): update rust crate memchr to 2.4.1 [\#98](https://github.com/sarub0b0/kubetui/pull/98) ([sarub0b0](https://github.com/sarub0b0))
- chore\(deps\): update all dependencies [\#97](https://github.com/sarub0b0/kubetui/pull/97) ([sarub0b0](https://github.com/sarub0b0))

## [v0.5.1](https://github.com/sarub0b0/kubetui/tree/v0.5.1) (2021-10-06)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.5.0...v0.5.1)

## [v0.5.0](https://github.com/sarub0b0/kubetui/tree/v0.5.0) (2021-10-06)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.4.4...v0.5.0)

**Closed issues:**

- 選択中のnamespaceやapiの状態を保持する機能を実装（contextを戻した時に以前の状態を作りたい） [\#94](https://github.com/sarub0b0/kubetui/issues/94)
- クラスタ切り替え機能 [\#78](https://github.com/sarub0b0/kubetui/issues/78)
- 起動に失敗しているpodのログを取得しようとしたとき関連するイベント情報を出力する [\#75](https://github.com/sarub0b0/kubetui/issues/75)
- ログ取得を実行したときContainerCreatingならRunningになるまで待機する処理を追加する [\#64](https://github.com/sarub0b0/kubetui/issues/64)

**Merged pull requests:**

- Restore kube state [\#96](https://github.com/sarub0b0/kubetui/pull/96) ([sarub0b0](https://github.com/sarub0b0))
- chore\(deps\): update rust crate pretty\_assertions to v1 [\#93](https://github.com/sarub0b0/kubetui/pull/93) ([sarub0b0](https://github.com/sarub0b0))
- fix\(deps\): update rust crate nom to v7 [\#92](https://github.com/sarub0b0/kubetui/pull/92) ([sarub0b0](https://github.com/sarub0b0))
- Change context [\#91](https://github.com/sarub0b0/kubetui/pull/91) ([sarub0b0](https://github.com/sarub0b0))
- 起動に失敗しているpodのログを取得しようとしたとき関連するイベント情報を出力する [\#90](https://github.com/sarub0b0/kubetui/pull/90) ([sarub0b0](https://github.com/sarub0b0))
- fix\(deps\): update kube-rs [\#89](https://github.com/sarub0b0/kubetui/pull/89) ([sarub0b0](https://github.com/sarub0b0))
- Update Rust crate unicode-segmentation to 1.8 [\#88](https://github.com/sarub0b0/kubetui/pull/88) ([sarub0b0](https://github.com/sarub0b0))
- Update Rust crate nom to 6.2.1 [\#87](https://github.com/sarub0b0/kubetui/pull/87) ([sarub0b0](https://github.com/sarub0b0))
- Update Rust crate crossterm to v0.21.0 [\#86](https://github.com/sarub0b0/kubetui/pull/86) ([sarub0b0](https://github.com/sarub0b0))

## [v0.4.4](https://github.com/sarub0b0/kubetui/tree/v0.4.4) (2021-07-04)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.4.3...v0.4.4)

## [v0.4.3](https://github.com/sarub0b0/kubetui/tree/v0.4.3) (2021-07-04)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.4.2...v0.4.3)

**Fixed bugs:**

- nodes.metrics.k8s.ioのようなtableデータを取得できないリソースへの対応 [\#74](https://github.com/sarub0b0/kubetui/issues/74)

## [v0.4.2](https://github.com/sarub0b0/kubetui/tree/v0.4.2) (2021-07-02)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.4.1...v0.4.2)

**Fixed bugs:**

- マウスでテーブルウィジェットのアイテム選択が正しく行われない問題を修正 [\#72](https://github.com/sarub0b0/kubetui/issues/72)

**Closed issues:**

- k8sにAPIをたたくスレッドでエラーがでたときにviewにエラー内容を出力できるようにする [\#63](https://github.com/sarub0b0/kubetui/issues/63)

**Merged pull requests:**

- feat\(readme\) キーバインドの説明を追加 [\#71](https://github.com/sarub0b0/kubetui/pull/71) ([sarub0b0](https://github.com/sarub0b0))

## [v0.4.1](https://github.com/sarub0b0/kubetui/tree/v0.4.1) (2021-06-27)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.4.0...v0.4.1)

## [v0.4.0](https://github.com/sarub0b0/kubetui/tree/v0.4.0) (2021-06-21)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.3.3...v0.4.0)

**Closed issues:**

- 複数のネームスペースを選択可能にする [\#51](https://github.com/sarub0b0/kubetui/issues/51)

## [v0.3.3](https://github.com/sarub0b0/kubetui/tree/v0.3.3) (2021-06-10)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.3.2...v0.3.3)

**Closed issues:**

- コマンド引数で分割の方向を切り替えられるようにする [\#70](https://github.com/sarub0b0/kubetui/issues/70)
- レイアウトが崩れる問題を修正 [\#57](https://github.com/sarub0b0/kubetui/issues/57)

## [v0.3.2](https://github.com/sarub0b0/kubetui/tree/v0.3.2) (2021-06-10)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.3.1...v0.3.2)

## [v0.3.1](https://github.com/sarub0b0/kubetui/tree/v0.3.1) (2021-06-09)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.3.0...v0.3.1)

## [v0.3.0](https://github.com/sarub0b0/kubetui/tree/v0.3.0) (2021-06-06)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.3.0-alpha...v0.3.0)

**Closed issues:**

- Homeキーで一番上, Endキーで一番下に移動できるようにする [\#68](https://github.com/sarub0b0/kubetui/issues/68)
- マウスイベントに対応 [\#65](https://github.com/sarub0b0/kubetui/issues/65)

## [v0.3.0-alpha](https://github.com/sarub0b0/kubetui/tree/v0.3.0-alpha) (2021-06-03)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.2.1...v0.3.0-alpha)

**Closed issues:**

- ポップアップを閉じるキーをESCに変更 [\#66](https://github.com/sarub0b0/kubetui/issues/66)

**Merged pull requests:**

- マウスイベントに対応 [\#67](https://github.com/sarub0b0/kubetui/pull/67) ([sarub0b0](https://github.com/sarub0b0))

## [v0.2.1](https://github.com/sarub0b0/kubetui/tree/v0.2.1) (2021-05-25)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.2.0...v0.2.1)

## [v0.2.0](https://github.com/sarub0b0/kubetui/tree/v0.2.0) (2021-05-18)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/v0.1.0...v0.2.0)

**Fixed bugs:**

- 画面サイズ変更時クラッシュする問題の解決 [\#37](https://github.com/sarub0b0/kubetui/issues/37)

**Closed issues:**

- HOME, ENDキー対応 [\#62](https://github.com/sarub0b0/kubetui/issues/62)
- ターミナルの背景が白だとペインタイトルの文字色が白で読めない [\#61](https://github.com/sarub0b0/kubetui/issues/61)
- 色に関する制御文字の対応範囲を広げる（22とか） [\#60](https://github.com/sarub0b0/kubetui/issues/60)

## [v0.1.0](https://github.com/sarub0b0/kubetui/tree/v0.1.0) (2021-05-16)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/0.1.6...v0.1.0)

**Closed issues:**

- windows環境でビルドが通るようにActions修正 [\#58](https://github.com/sarub0b0/kubetui/issues/58)
- 複数アイテム選択コンポーネントでCtrl-k,wを追加する [\#55](https://github.com/sarub0b0/kubetui/issues/55)
- アイテム選択時にフィルターを初期化しないようにする [\#54](https://github.com/sarub0b0/kubetui/issues/54)

## [0.1.6](https://github.com/sarub0b0/kubetui/tree/0.1.6) (2021-05-15)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/0.1.4...0.1.6)

## [0.1.4](https://github.com/sarub0b0/kubetui/tree/0.1.4) (2021-05-14)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/0.1.3...0.1.4)

**Closed issues:**

- イベントタブをfollowにする [\#56](https://github.com/sarub0b0/kubetui/issues/56)

## [0.1.3](https://github.com/sarub0b0/kubetui/tree/0.1.3) (2021-05-14)

[Full Changelog](https://github.com/sarub0b0/kubetui/compare/d39f48c66deade9aee37680bcafb236752e06fc2...0.1.3)

**Fixed bugs:**

- 画面サイズ変更時にクラッシュする問題を修正 [\#32](https://github.com/sarub0b0/kubetui/issues/32)
- ConfigMap等の表示で見切れる問題を修正 [\#25](https://github.com/sarub0b0/kubetui/issues/25)

**Closed issues:**

- multiple-selectのItemsとSelectedの方向を変えられるようにする [\#53](https://github.com/sarub0b0/kubetui/issues/53)
- APIsタブでいくつかのカラムが表示されない問題を修正 [\#52](https://github.com/sarub0b0/kubetui/issues/52)
- APIsをapi-groupでソートする [\#50](https://github.com/sarub0b0/kubetui/issues/50)
- スクロールができるウィジェットのデータ更新時にクラッシュする問題を修正 [\#49](https://github.com/sarub0b0/kubetui/issues/49)
- イベントリソースを時間でソートする [\#48](https://github.com/sarub0b0/kubetui/issues/48)
- サブウィンドウを見ている時にイベントをこぼしてしまう問題を修正 [\#47](https://github.com/sarub0b0/kubetui/issues/47)
- textウィジェットで折り返しするかを指定できる機能を実装 [\#46](https://github.com/sarub0b0/kubetui/issues/46)
- 複数のアイテム選択ができるコンポーネントを作る [\#45](https://github.com/sarub0b0/kubetui/issues/45)
- age等の時間がずれないようにする [\#44](https://github.com/sarub0b0/kubetui/issues/44)
- 指定したリソースのwatch結果表示機能 [\#43](https://github.com/sarub0b0/kubetui/issues/43)
- namespace切り替え時に情報が更新されないことがある問題を修正 [\#42](https://github.com/sarub0b0/kubetui/issues/42)
- クラッシュ時にターミナルモードを元に戻す際にカーソルを表示にする [\#40](https://github.com/sarub0b0/kubetui/issues/40)
- k8s-event情報の逐次追加とイベント作成時間の更新を両立する [\#39](https://github.com/sarub0b0/kubetui/issues/39)
- 日本語などの全角文字に対応 [\#38](https://github.com/sarub0b0/kubetui/issues/38)
- \x1b\[\<xxx\>m以外の制御文字に対応 [\#36](https://github.com/sarub0b0/kubetui/issues/36)
- ステータスバーのスクロールとテキスト行数描画の対象にConfigsを含める [\#35](https://github.com/sarub0b0/kubetui/issues/35)
- u16::MAX以上のログに対応 [\#34](https://github.com/sarub0b0/kubetui/issues/34)
- wrap計算時に制御文字を含めないようにする [\#33](https://github.com/sarub0b0/kubetui/issues/33)
- textで太字などに対応する [\#31](https://github.com/sarub0b0/kubetui/issues/31)
- ログ取得関数のタイムアウトエラー処理 [\#30](https://github.com/sarub0b0/kubetui/issues/30)
- channelのエラー処理を実装する [\#28](https://github.com/sarub0b0/kubetui/issues/28)
- 現在のクラスターとネームスペースの名前を表示 [\#26](https://github.com/sarub0b0/kubetui/issues/26)
- ConfigMap、Secretを表示できるタブを追加 [\#24](https://github.com/sarub0b0/kubetui/issues/24)
- 複数コンテナをもつpodのログを出力する [\#23](https://github.com/sarub0b0/kubetui/issues/23)
- podの起動に失敗した時にeventの内容を表示する [\#22](https://github.com/sarub0b0/kubetui/issues/22)
- podのステータスでerror, crash, completeなどに対応  [\#21](https://github.com/sarub0b0/kubetui/issues/21)
- viewからロジックの処理を追い出す [\#19](https://github.com/sarub0b0/kubetui/issues/19)
- ステータスバーにlogの現在位置を表示する機能を実装 [\#18](https://github.com/sarub0b0/kubetui/issues/18)
- 借用している箇所を見直す [\#17](https://github.com/sarub0b0/kubetui/issues/17)
- Into\<String\>に置き換える [\#16](https://github.com/sarub0b0/kubetui/issues/16)
- ログに色がついている場合に色をつける機能を実装 [\#15](https://github.com/sarub0b0/kubetui/issues/15)
- スクロールの最大値を計算する処理を実装 [\#14](https://github.com/sarub0b0/kubetui/issues/14)
- 別のコンテナのログを表示する際にカーソルを初期値に戻す機能を実装 [\#13](https://github.com/sarub0b0/kubetui/issues/13)
- ログが多いとスクロールに時間がかかる問題への対策 [\#12](https://github.com/sarub0b0/kubetui/issues/12)
- shit-tabで逆回りにpaneを切り替える機能 [\#11](https://github.com/sarub0b0/kubetui/issues/11)
- イベント情報の取得と表示機能を実装 [\#10](https://github.com/sarub0b0/kubetui/issues/10)
- ctrl-n, ctrp-pでの移動を実装 [\#9](https://github.com/sarub0b0/kubetui/issues/9)
- クラスタ切り替え機能を実装 [\#8](https://github.com/sarub0b0/kubetui/issues/8)
- namespace切り替え機能の実装 [\#6](https://github.com/sarub0b0/kubetui/issues/6)
- 非同期でpodの情報を取得する機能を実装 [\#5](https://github.com/sarub0b0/kubetui/issues/5)
- 選択中のPodのログを取得、表示する機能を実装 [\#4](https://github.com/sarub0b0/kubetui/issues/4)
- podのリストを取得してアイテムリストに表示する機能の実装 [\#3](https://github.com/sarub0b0/kubetui/issues/3)
- 選択中アイテムの文字色を暗い色に変更する [\#2](https://github.com/sarub0b0/kubetui/issues/2)
- フォーカスしているブロックのタイトルを強調する機能を実装する [\#1](https://github.com/sarub0b0/kubetui/issues/1)



\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*
