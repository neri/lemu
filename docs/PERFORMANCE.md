# 性能の判断記録と測定方法

採用・不採用を決めた最適化の結論と、その測定方法をまとめる。数値は当時の実装・環境で測ったもので、現在のコードで再測定した値ではない。同じ案を再検討するときは、ここに書いた不採用の理由を先に確認する。

## 1. 測定方法

### 基本の進め方

- 正しさを先に確認し、同じ入力と同じ環境で変更前後を測る。時間測定とノイズは通常テストの失敗条件にしない。
- 製品の主な実行環境は Wasm。native と Wasm の両方を測り、native の中央値だけを根拠に高速化を主張しない。
- Wasm と native 計測の release 設定は `opt-level="s"`、`lto=true`、`codegen-units=1`、`panic="abort"`。
  - workspace の Cargo.toml には書いていないので、`cargo build --release` だけではこの設定にならない。`scripts/bench/prepare-plum-rmw-bench.py` は一時的な workspace にこの設定を書き込む。

### 入力

| 入力 | 用途 |
| --- | --- |
| [fixtures/frequency-v1](../fixtures/frequency-v1/) | 一般的な整数コードの頻度に合わせた 7 系統 × O2/Os の 14 ケース。`frequency_bench` / `flat_frequency_bench` が使う |
| `plum/examples/support/bench/floating-flags/guest/` | x87 / SSE2 の浮動小数点 workload |
| `plum/examples/support/simd/` | `simd_bench` の整数 XMM・浮動小数点・MMX ループ |

コマンドは [scripts/README.md](../scripts/README.md) を参照する。`make test-tools` は固定 guest の hash と集計処理を検証する。

### ノイズの扱い

- ハイブリッド CPU では P コアに固定し（例：`taskset -c 6`）、変更前後を交互に実行する。同じビルド同士の差（約 ±2〜3%）も測っておく。
- dispatcher（`_execute`）の配置だけで数%〜10%動く。バイト単位で同一の `_execute` が 48 バイトずれて 7〜8% 変わった例がある。
- 数%の差を判断するときは、`-C llvm-args=-align-all-functions=4..7` のアラインメント掃引で分布を比べる。
- 全アラインメントで遅くても、それだけで実コストの証拠にはならない。`-align-all-functions` は関数の先頭しか動かさず、`_execute` 内部のブロック配置は固定のままだからである。次も確認する。
  - 遅いケースが変更したコードを実際に通るか。
  - 変種を変えても同じケースが遅いか。
  - 変更していないエンジン（対照）と幾何平均がどう動くか。
  - Wasm の A/B でも同じ傾向か。
- `nm -S -C <bench> | grep -E "_execute|branch_to"` で dispatcher のサイズとインライン化を比べる。`branch_to` が独立したシンボルになったり、`_execute` のサイズが大きく変わったりしたら要注意。
- `ProcessorState`、`SystemRegisters` などエンジン間で共有する構造にフィールドを足すと、両エンジンの `_execute` が変わる。この場合、どちらのエンジンも対照にならない。

## 2. 採用した構成

### dispatcher とコード配置

- **Uop の大きさ：** `Uop` は 8 バイト以下（コンパイル時に検査）。頻出命令は主 `Uop`、低頻度命令は `Uop::Minor` に置く。
  - 低頻度の操作に主 variant を足すと、使わないケースまで遅くなる。
- **cold 化：** 実行頻度の低い helper は `#[cold] #[inline(never)]` にする。
  - helper が大きくなると、LLVM のインライン予算から押し出されて、`branch_to`、`tick`、`flush_pending` のような分岐経路のコードが `_execute` の外へ出てしまう。`reset_caches` の肥大化で fib が約 10% 遅くなった例がある。
  - 逆に、ホットなメモリ入口（`read_memory*`、`rmw_read*`、`write_bytes`）は `#[inline(always)]` にし、遅い経路だけを cold にする。
- **SIMD の実行関数：** `execute_simd` は cold・非インライン。`execute_fpu` へインライン展開されると、`sse` ビルドの FlatUME のメモリ中心のケースが全アラインメントで 5〜17% 遅くなった。
- **`ProcessorState` のレイアウト：** 全ビルドで `repr(C)` にし、`gpr` をオフセット 0 に固定する。
  - Rust の既定レイアウトでは、`simd` を足したときに `gpr` がオフセット 304 へ動いた。全レジスターアクセスに 32 ビット変位が付き、LLVM が dispatcher の間接ジャンプを各ハンドラーの末尾に複製しなくなった（260 → 42 か所）。
  - 宣言順は以前コンパイラが選んでいた並びに合わせ、`simd` は末尾に置く。
- **lazy flags の helper：** `_execute` にインライン展開される `LazyOp` / `FlagsRegister` の helper には、panic や `unreachable!()` の経路を置かない。dispatcher の内部配置が変わり、無関係なケースの時間まで全アラインメントで 5〜13% 動いた。
- **enum と match の並び：** 可読性を優先してアルファベット順にする。`execute_misc` の分離はやめ、Minor の本体は内側の match に置く。Wasm の flags/BCD/XLAT workload は改善した。native は比較構成によって数%〜十数%動くが、この並びを維持する。
- **Minor の分離：** 外側 `Uop` と `Uop::Minor` の二段構成を維持する。全 Minor を統合した配置は、native / Wasm とも fib で遅かった。

### 命令変換と融合

- **RMW の集約：** 通常メモリの更新を `emit_memory_update` に集約した。`[base+0]` の EA 準備は 0 Uop、それ以外は 1 Uop。頻度の低い専用 RMW 入口は、頻度の再調査で削った。
- **メモリ即値演算：** ADD/SUB/AND/OR/XOR32 と ADD/SUB8 の 7 ケースは、load → 即値 ALU → store の 3 Uop を、定数 load → 既存 RMW の 2 Uop にした。Wasm で対象形式が改善し、guard workload に退行がなかった。
- **MOVZX/MOVSX：** base のみのアドレスからの load と拡張を、幅別の 2 入口で融合した。
- **Jcc：** ZF だけを見る条件は `Jz` / `Jnz` に特殊化した。
- **RET target cache：** TraceDecoder ごとに 16 エントリーの direct-mapped 配列を持つ。索引は EIP の下位 4 ビット、追加の保持量は 128 バイト。1 / 4 / 16 エントリー、xor 索引、miss 経路の cold 化を比べたうえでの選択。
- **SimpleFetcher：** `fetch_u16` / `fetch_u32` はスライスからまとめて読む。デコード時間は native で −2〜−10%、Wasm で −8〜−18%。

### 浮動小数点と SIMD

- **フラグ評価の省略：** FCOMI/(U)COMIS 系は上書きする整数フラグを全評価せず、比較結果をまとめて設定する。FCMOV は必要な条件だけ評価する。SSE の通常値の高速経路では、既に立っている PE を検出し直さない。
- **SIMD の整数・論理演算：** 型付きの固定長配列で書き、コンパイラの自動ベクトル化に任せる。x86-64 では SSE2、Wasm では simd128、AArch64 では NEON の命令になることを確認した。`core::arch` による明示的な実装は使わない。
  - 形式ごとの処理は、コンパイル時の表（`PLANS`）を 1 回引くだけにした。
- **SIMD の浮動小数点算術：** 全レーンが既定経路の条件を満たすとき、全レーンをまとめてホストで計算する（`float::arith_lanes`）。

### 任意で有効にするもの

- **`flag-elision` feature：** trace 内で、後続命令が読む前に上書きされる整数フラグの更新を省く。既定では無効。

### FlatUME の書き換え検知

- 監視の単位は 4 KiB ページで、デコード済みページへの書き込みがあると全キャッシュを破棄する。
- 4,096 関数をキャッシュした状態での 1 回の書き込みと実行の費用（native）：別ページのデータ更新は約 0.17 ms、同じページのデータ更新とコード更新は約 4.4 ms（再デコードと再実行を含む）。

## 3. 不採用・見送り

| 案 | 理由 |
| --- | --- |
| JZ/JNZ 以外の Jcc 専用 Uop（JC/JNC、JBE/JA、JL/JGE、JLE/JG） | 対象ループは短縮したが、carry の追加で fib が約 9%、全段階の累積で約 8% 遅くなった。P コア固定の再測定でも退行が残った |
| 汎用 `eval_cc` の JLE/JG を ZF で短絡 | 等値ケースは約 22% 速いが、Wasm の非等値ケースが約 5% 遅い |
| 8/16 ビット load の EA 計算と読み取りの融合 | 3 案とも通常経路に退行が残った |
| 上記以外の load と拡張の融合候補（初回の 6 候補） | 通常経路の退行 |
| 全 Minor の統合 | fib が native / Wasm とも遅い |
| `address_map` の 4 KiB ページ密表 | 検索だけなら約 10 倍速い（8 ns → 0.8 ns）が、全体では配置ノイズを超える差がなく、メモリは BTreeMap の 2〜3 倍。検索がボトルネックと測れたら、1 KiB ページから再検討する |
| 専用 RMW の範囲検証を 1 回にまとめる helper | store 側の重複検証はコンパイラが既に除去していた |
| x87 例外状態と MXCSR の遅延評価 | 整数 EFLAGS のように最新の演算を 1 つ保存する方式を、そのまま適用できない。省略版（上記）で十分とした |
| SIMD の `core::arch` 明示実装 | 自動ベクトル化で同等の命令になり、残る時間は形式の振り分けとエンジンの経路にある |
| ARM64 での MMX 読み込みの直接化 | 命令列は短くなるが速くならなかった（HarvardUME の mmx は 7.5% 遅い） |

## 4. 未着手の候補

- **UPC から EIP への逆引き：** 例外時の EIP 復元はアドレスマップの走査で、最悪 O(n)。HLE や RDTSC が多く、デコード済み命令の多いアプリで効く可能性がある。速度は未測定。
- **キャッシュ済み実行とデコード・解決の経路分離：** 生成コードと A/B の確認が必要。
- **FlatUME の Wasm 性能：** 分離直後の測定では、14 負荷の幾何平均で HarvardUME より 6.1% 遅く、calls-Os は 11.8% 遅かった。
