# Linux/i386 ELFで統合メモリを確認する

静的リンクしたfreestandingのELF32を、ホストLinuxと `FlatUME` の両方で実行する確認用アプリと、小さなLinux ABIランナー。ゲストはlibcを使わず、エミュレーター側ではカーネルの代わりに必要なsyscallだけをRustで処理する。

## ビルドと実行

workspaceのrootで実行する。GCCとbinutilsのi386対応が必要。32bitのlibcは不要。

```sh
sh plum/examples/linux32/build.sh
printf '6 7 5\n' | cargo run -p plum --release \
  --example linux32 --offline -- target/linux32/jit-calc --stats
```

出力:

```text
initial: 37
patched: 44
replaced: 99
```

アプリ `jit_calc.c` はstdinから係数a、定数b、入力x（各0〜999999）を読み、次を実行する。計算は32bit unsignedのwraparound。

1. `mmap2` で確保した領域へ `a*x+b` を計算する16byteの機械語を書き、64回呼び出す。
2. 同じ領域の即値をa+1、b+2へ書き換え、64回呼び出す。
3. 同じアドレスへ `return 99` の6byte関数を書き、命令長が変わる置き換えを確認する。
4. stdinに6byte追加されていれば、`read` でそのbyteをキャッシュ済み関数へ直接読み込み、再度呼び出す。

4番目のケースは次で確認できる。最後の出力が `host patched: 123` となる。

```sh
printf '6 7 5\n\270\173\000\000\000\303' | \
  target/release/examples/linux32 target/linux32/jit-calc
```

同じ `target/linux32/jit-calc` は、i386 ABIを実行可能なLinuxなら直接実行して比較できる。今回のホストでも通常ケースとreadによる置き換えの両方で出力が一致した。実行環境が32bit syscallを遮断する場合は、エミュレーター側での確認だけを行える。

## Wasmでも同じELFを使う

```sh
cargo build -p plum --release \
  --target wasm32-unknown-unknown --example linux32 --offline
node scripts/test/linux32-wasm.mjs
```

4種類の整数入力（overflowを含む）、readによるコード更新、不正ELFの拒否の計6ケースを検証する。WasmへELFとstdinのbyteを渡し、stdout/stderrと終了コードを受け取るため、ゲストの再コンパイルは不要。

## 対応範囲

- ELF32 / little endian / EM_386 / ET_EXEC、PT_LOADのロード、BSSのゼロ初期化。
- entryを実行可能PT_LOAD内で検査。PT_INTERP・PT_DYNAMIC・PIE・範囲外/切れたsegmentは拒否。
- 初期stackはargc=1、argv、空envp、AT_PAGESZ・AT_ENTRY・AT_NULL。
- `int 0x80` の `read` (3、stdinのみ)、`write` (4、stdout/stderrのみ)、`mmap2` (192、anonymous/private/RWXのみ)、`exit` (1)、`exit_group` (252)。
- 未対応syscallは `-ENOSYS`、不正fd/アドレス/確保要求はLinux形式の負のerrnoを返す。
- syscallによるゲストメモリ変更は必ず `write_bytes` を使い、コード書き換えを通知する。

32MiBの連続arenaと1MiBのstack予約を使う。mmapは単調増加で、munmapやmprotectは未対応。ELFのsegment権限や未マップ領域へのアクセス制限、NXはCPUへ強制していない。ページ保護、TLS、ファイルシステム、プロセス、シグナル、動的リンク、libcは対象外。stdinは最初にまとめて読み、stdout/stderrも終了時にまとめて返す。対話端末は実装していない。

出力は各fd最大1MiB、syscall数は最大4096。CPUの命令数による打ち切り機能はなく、無限ループを停止する仕組みはこのrunnerにはない。一般的なLinuxアプリや非信頼コードを安全に実行する環境ではなく、統合メモリをLinux ABI経由で確認するためのもの。

## 回帰検証

```sh
cargo test -p plum --test linux32 --offline
```

2026-11-17の再構築で旧4テストを削除し、手で構成した小さなELFによる新2契約へ置換した。
L01は切れたheader・PT_INTERP・address overflowの拒否、L02はread→writeと未対応syscallのENOSYSを確認する。
nativeテストにGCCは不要で、4 feature構成で各2件を完全実行する。
JIT/host readによるコード更新とSSE2利用例の受け入れは`make test-wasm`で別に確認する。
テストの範囲は[docs/TESTING.md](../../../docs/TESTING.md)を参照。

ELFローダーとABIは `examples/support/linux32.rs`、CLI/Wasm入口は `examples/linux32.rs`。どちらもサンプル専用で、従来 `HarvardUME` や統合版の実行ループは変更していない。


## 今回の検証結果（2026-10-18）

- 同一ELFのホストLinux / native FlatUME / Wasmで、`37 → 44 → 99` と、readで置き換えた後の `123` が一致。
- Wasmは6ケース成功。RustのLinux ABI統合テストは3件成功（内部で複数の入力・ELF不正・errnoケースを確認）。
- workspace回帰は従来構成72件、統合版有効107件、全機能157件成功。従来のignored 1件のみで除外指定なし。

## SSE2 版

`sse_calc.c` は gcc で SSE2 向けにビルドする（`-O3 -msse2 -mfpmath=sse`）freestanding アプリで、ベクトル化されたループ、倍精度の演算、変換、丸めモード、NaN の結果を16進のビットで出力する。ビルド手順と期待出力は `plum/examples/support/simd/README.md` にある。`--features sse` の `linux32` で実行すると、x86 CPU と同じ出力になる。

