# テストの実行・配置・追加方針

テストは少数の契約で構成する。全命令・全組み合わせの網羅はしない（未検証の範囲は 5 章）。

## 1. 実行入口

リポジトリのルートで実行する。入口は Make にあり、mise は `test`（`make test`）などの一部だけを持つ。依存を取得した後は `CARGO_NET_OFFLINE=true make test` などで実行できる。

| コマンド | 範囲 |
| --- | --- |
| `make test` / `make test-native` | workspace の既定構成と PLUM の 4 構成。example のビルドも検証する |
| `make test-common` | 共通部品の公開 API と private 部品 |
| `make test-harvard` | HarvardUME の接続・固有契約 |
| `make test-flat` | FlatUME の接続・固有契約 |
| `make test-matrix` | PLUM の no-default / x87 のみ / default / all-features と example のビルド |
| `make test-wasm` | 小さな UME の契約・数値契約と Linux32 の利用例を Node で検証する |
| `make test-tools` | 性能集計の重み・ノイズ判定と、固定ゲストの復号後の hash |

- Rust nightly が必要。native の Linux32 の 2 契約は GCC 不要。
- Wasm で Linux32 の JIT 利用例をビルドするには、Linux x86_64 と i386 をリンクできる GCC / binutils が要る。
- Wasm には `wasm32-unknown-unknown` target、Node、Python 3 も必要。
- 実機との照合（oracle）は現在ない。

## 2. 配置と責務

| 配置 | 責務 |
| --- | --- |
| `ir86/tests/contracts.rs` | デコーダーと fetch の最小契約（3 章） |
| `plum/tests/common.rs` | GPR、状態、ALU、遅延フラグ・条件コードの公開 API |
| `plum/src/tests/common_contracts.rs` | BCD 調整、除算、文字列のバイト操作、x87 / SIMD の計算・codec・状態、profile 集計 |
| `plum/tests/harvard.rs` | HarvardUME の lowering、コード / データ分離、フォールトと再開、数値命令のアダプター |
| `plum/tests/flat.rs` | FlatUME の統合メモリ、ゲスト / ホストからのコード更新、フォールトと再開、数値命令のアダプター |
| `plum/tests/{harvard,flat}/profile.rs` | profile の接続契約。`uop-profile` のときだけ親から登録する |
| `plum/tests/flag_elision.rs` | `flag-elision` feature の契約 |
| `plum/tests/support/` | レジスターの初期設定と snapshot、TSV の読み取り、HarvardUME / FlatUME 共通の整数命令ゲスト（85 個の結果スロットと固定の期待値） |
| `plum/tests/linux32.rs` | 3 区分とは別の 2 契約 L01 / L02。小さな ELF の拒否と read / write / ENOSYS の接続。GCC 不要 |
| `plum/examples/support/contracts/` | native と Wasm で共有する少数の固定ゲストと独立した期待値 |
| `plum/examples/support/bench/`、`plum/examples/support/simd/` | 計測 workload と、SIMD・Linux32 利用例の入力と期待出力 |
| `scripts/test/plum.py` | 3 区分の feature 別の予定件数を照合するランナー |

- 共通部品では計算そのものを検証する。各エンジンでは、別々に実装した lowering、メモリアクセス、確定の順序、例外の変換、再開への接続を検証する。数値表を全エンジンへ一律に展開しない。
- private なテストは `common_` 接頭辞で抽出する。テストのために製品 API の可視性を広げない。

## 3. IR86 の契約

`cargo test -p ir86` の 13 件。ホストにも feature にも依存しない。期待値は命令バイト列を手で解釈した固定値で、デコーダーや CSV から生成しない。検証するのは IR への変換までで、実行の正しさは保証しない。

| ID | 守る仕様 |
| --- | --- |
| ir01 | オペランド幅、重複した 66、ModR/M の reg / rm、上位 8 ビットレジスター |
| ir02 | imm8 の符号拡張 |
| ir03 | A16、disp8 の符号拡張、変位だけの形式、オペランド幅とアドレス幅の独立 |
| ir04 | A32 / SIB、base なし、index なし、セグメント上書き、67 |
| ir05 | 次の EIP を基準とする相対分岐と、16 / 32 ビットでの折り返し |
| ir06 | プレフィックス状態の命令間での分離と、モードの切り替え |
| ir07 | REP / REPNE、LOCK の消費、文字列命令の幅・セグメント・アドレス属性 |
| ir08 | システム命令と割り込みの識別（CR / DR、CC と CD 03） |
| ir09 | SIMD の必須プレフィックス、レジスターの方向、メモリ / imm8 |
| ir10 | UD でも既知の形式の末尾まで消費する |
| ir11 | x87 のメタデータと、オペランド幅・アドレス幅の独立 |
| ir12 | 各段階での入力の途切れと Fetch エラーの伝播 |
| ir13 | fetch のリトルエンディアン、符号付き値、範囲外、base の折り返し |

意図的に検証しないもの：
- ModR/M × x87 escape × モード × プレフィックスの全組み合わせ。
- SIB 内部表の全組み合わせ。
- 全 SIMD variant の往復と、全プレフィックス順序。
- 全 Jcc と LOOP / JCXZ の幅の組み合わせ、far 転送、ディスクリプタ / セグメント命令の全種。

## 4. feature 別の予定件数

`plum` は crate 単位で実行する。workspace 全体への feature 指定で条件が混ざるのを避ける。`scripts/test/plum.py` が次の件数を照合する。

| 構成 | 共通（公開 / private） | Harvard | Flat | 合計 |
| --- | ---: | ---: | ---: | ---: |
| no-default | 9 / 4 | 26 | 20 | 59 |
| x87 のみ | 9 / 10 | 26 | 20 | 65 |
| default | 9 / 19 | 27 | 21 | 76 |
| all-features | 9 / 20 | 30 | 26 | 85 |

- all-features ではこのほかに `flag_elision` の 10 件を、どの構成でも Linux32 の 2 件を実行する（上表の合計には含めない）。
- 表の値はテスト関数の数で、ループ内の入力数やカバレッジではない。
- ランナーは成功件数に加えて失敗・無視・filter の件数も照合し、0 件での成功を防ぐ。
- `uop-profile` を有効にすると、命令 EIP ごとの試行数、フォールトと再試行、REP の完了要素数、FlatUME のキャッシュ破棄後の累積行と最新の IR を検証する。
  - 個別には `cargo test -p plum --all-features --test harvard h07_` と `--test flat f07_` を使う。
  - FlatUME の profile は、キャッシュ破棄前の命令情報を EIP ごとに保持する。カウンターは累積、IR / Uop 列はその EIP の最新のデコードを表す。

## 5. 未検証の範囲

次は検証していない。必要になったら、守る契約と独立した期待値を示して、担当区分へ最小のケースを追加する。少数の契約の成功やエンジン間の一致を、全機能の検証と見なさない。

- 命令の全形式・全幅・全レジスター別名、REP 比較系の全停止条件とフォールト位置。
- x87 の全丸め・超越関数・PC 精度と例外の組み合わせ、SIMD の変換と保存・復元イメージの全種。
- profile の全フォールト種別と全自己書き換え経路、Wasm での profile、大規模キャッシュの stress。
- ブラウザでの実行。

## 6. Wasm

native と同じ固定入力・期待値で次を検証する（Node での実行で、ブラウザとは区別する）。

- LEAVE：HarvardUME / FlatUME 各 1 ケースで、フレーム境界のフォールトと修復後の再開。x87 / SSE の有無の両方。
- RMW：XADD のアドレス別名 1 ケース。
- load と拡張：メモリ末尾の符号拡張 1 ケース。
- 8 ビット load：AH への書き込みで上位ビットを保持する 1 ケース。
- FlatUME：ホストによるキャッシュ済みコードの更新 1 ケース（x87 の有無の両方）。
- SIMD：halfway の丸めと signaling NaN を各エンジン 2 ケース。通常の Wasm と simd128 の両方。
- Linux32：6 シナリオと、gcc で生成した SSE2 プログラムの固定出力。

## 7. ケースの追加と性能測定

- ケースには契約 ID（C / H / F）、入力、採用理由、期待値の根拠、feature / ホストの条件を書く。
- 境界・フォールト・再開の意味が異なるケースを優先し、総当たりや同じ計算の重複を増やさない。
- エンジン間の一致だけを oracle にしない。未定義のフラグや CPU 依存の値を固定しない。
- 失敗時に契約 ID と入力が分かる assert を使う。
- 性能の測定方法は [PERFORMANCE.md](PERFORMANCE.md)、固定ゲストは [fixtures/README.md](../fixtures/README.md) を参照する。
