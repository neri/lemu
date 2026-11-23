# ドキュメント案内

| 資料 | 内容 |
| --- | --- |
| [設計](DESIGN.md) | IR86 と 2 種類の実行エンジンの構成、実行モデル、HarvardUME / FlatUME、共通実装、x87 / SIMD、ホスト連携、制約 |
| [テスト](TESTING.md) | 実行コマンド、テストの配置、予定件数、未検証の範囲、Wasm での検証 |
| [性能](PERFORMANCE.md) | 測定方法と、採用・不採用にした最適化の判断 |

## その他の資料

- `future/`：まだ実装していない案。
  - [IR86 x87 初期デコード移行計画](future/ir86-x87-decode-plan.md)
  - [FS/GS を使う OS・ランタイムへの対応調査](future/fs-gs-support-analysis.md)
  - [segment base の組み込み方式の比較](future/fs-gs-uop-comparison.md)

## 更新するとき

- 利用方法や制約を変えたら、該当する文書も更新する。
- 最適化を採用・不採用にしたら、理由を [PERFORMANCE.md](PERFORMANCE.md) に数行で残す。
- 固定ゲストの配置は [fixtures/README.md](../fixtures/README.md) に従う。
