# FS/GS を使う OS・ランタイムへの対応調査

> 現状（2026-11）: FlatUME は GS 相対の `disp32` ロードと間接 CALL に限って GS の base を使う（[DESIGN.md](../DESIGN.md) 6.2 節）。それ以外の FS/GS は未実装。

調査日: 2026-10-21。対象: `6222a00` の FlatUME、32bit protected-mode のアプリケーション向け。実装は変更していない。以下の工数・速度への評価はソース構造からの見積もりで、実行時ベンチマークではない。

## 結論

DS/ES/SS/CS は現在の flat な扱いを維持し、FS/GS にだけ base を適用する拡張は可能。IR デコーダーには方針を戻さず、FlatUME の lowering と実行処理で扱う。

推奨は、互換性を維持する既存の全 segment 無視モードと、FS/GS base を使うモードを FlatUME の構築時に選択する構成。base の値は実行時の状態から参照し、Uop に定数として埋め込まない。通常アクセスへ segment 判定を追加せず、FS/GS 付きアクセスだけを別のアドレス生成へ振り分ける。

ただし、base 加算、ゲストの selector load、OS の TLS 初期化、一般アプリ互換は別の対応範囲。前者だけで libc や Windows アプリが動くとは判断できない。

## 1. OS ごとの用途と今回の範囲

| 対象 | 代表的な用途 | 必要になるもの |
| --- | --- | --- |
| Linux/i386 | glibc の TLS は GS を使用 | GS base、TLS descriptor、`set_thread_area` と selector load、TCB/TLS 初期化 |
| Windows/x86 | FS 経由で TEB にアクセス | FS base、TEB の配置・必要な内容、周辺の Win32/NT ABI |
| Linux/x86-64 | FS による TLS | 本エミュレーターの32bit実行とは別の範囲 |
| Windows/x64 | GS による TEB アクセス | 同上。GS 対応だけで64bit命令は実行できない |

Linux/i386 の glibc 実装は `set_thread_area` 後に GS selector を load する。同じ selector 値でも descriptor 情報を再ロードする必要があると明記している。[glibc i386 TLS 実装](https://raw.githubusercontent.com/bminor/glibc/master/sysdeps/i386/nptl/tls.h)

Windows SDK の x86 `NtCurrentTeb` は FS:0x18 を読み、AMD64 側には GS による TEB アクセスがある。TEB の中身の再現は base 加算とは別作業。[Microsoft Windows SDK winnt.h](https://github.com/microsoft/win32metadata/blob/main/generation/WinSDK/RecompiledIdlHeaders/um/winnt.h)、[Microsoft TEB 定義](https://learn.microsoft.com/en-us/windows/win32/api/winternl/ns-winternl-teb)

Linux の x86-64 向け FS/GS、`arch_prctl`、FSGSBASE の説明を、そのまま i386 ABI に適用しない。今回の優先対象は i386 の selector/descriptor を使う方式。[Linux kernel FS/GS documentation](https://docs.kernel.org/arch/x86/x86_64/fsgs.html)

## 2. 現実装で利用できる部分と不足

| 箇所 | 現状 | 追加の要点 |
| --- | --- | --- |
| `ir86` | memory operand の segment、string source の segment を保持 | 基本的に変更不要。デコーダーが OS 方針を選ばない |
| `ProcessorState` / `SegmentRegister` | selector/base/limit/ar を既に持ち、`sr_mut()` で設定可能 | base 値の格納先は利用可能。ただし設定しても現在の FlatUME は無視する |
| `flat/tracer.rs` | segment によらず同じ load/store を生成 | FS/GS 付き operand だけ別のアドレス生成へ |
| `flat/mod.rs` | 通常の read/write はゲスト線形アドレスを受け取る | 公開メモリ API は線形アドレスのまま。segment を二重加算しない |
| `flat/string.rs` | packed segment 属性はあるが実行時は使わない | source の base を適用。index/進捗は offset のまま |
| `flat/fpu.rs` | segment 属性を保持し、保存用 selector に利用 | offset と linear address を区別して memory access/FDP を処理 |
| selector 操作 | IR は MOV/PUSH/POP segment、LFS/LGS 等を表現できるが、Flat lowering に対応 arm がない | ゲストが selector を設定・取得する経路は追加実装 |
| Linux sample | read/write/mmap2/exit 等の小さい ABI、TLS syscall なし | `set_thread_area` 等、必要な TLS 配置・初期化 |

`SegmentRegister::load_rm()` は selector×16 の real-mode 用で、Linux/i386 の TLS descriptor の解決には使えない。

現在の ELF loader は PT_LOAD を配置し、PT_INTERP/dynamic linking を拒否する。PT_TLS の専用処理はなく、auxv は最小限。TLS image/zero-fill、TCB、runtime が必要とする補助情報をどこが構築するかを別途決める必要がある。既存 sample は freestanding C 向けで、libc の起動を検証済みではない。

確認したファイル:

- [segment 状態](../../plum/src/gpr.rs)、[CPU 状態 API](../../plum/src/cpu.rs)
- [Flat lowering](../../plum/src/flat/tracer.rs)、[実行](../../plum/src/flat/mod.rs)
- [string](../../plum/src/flat/string.rs)、[x87](../../plum/src/flat/fpu.rs)、[memory/書き換え監視](../../plum/src/flat/memory.rs)
- [Linux ELF/ABI sample](../../plum/examples/support/linux32.rs)

## 3. 推奨するアドレス変換

32bit address form の base 拡張では、概念上次の順になる。

```text
logical_offset = register + index * scale + displacement  (32bit)
linear_address = logical_offset + FS.base / GS.base       (32bit)
allocation_offset = linear_address - guest_base
bounds validation → read/write → code-write notification
```

32bit の加算・wrap と、アクセス幅全体の範囲検査を分ける。例えば `GS:[0xfffffff4]` は負の TLS offset を表せるため、base を足す前に arena 外として拒否してはいけない。segment limit を含む厳密な protected-mode の fault は別の検査が必要。

base は**ホストポインターではなくゲスト線形アドレス**。TLS/TEB は既存の allocation 内に配置できる。これは別のメモリ領域を必須とする変更ではない。固定アドレスの TLS が既存 arena の外に必要な場合は、loader の再配置、arena の拡張、複数 region のいずれかが必要になり、別のコストになる。

### 通常アクセスの経路

lowering 時に「通常 segment」「FS/GS」を選別する。通常 segment は現在の fused load/store/RMW を維持する。新しい address Uop は flags を変えず、ゲスト register を一時アドレスとして上書きしない。

`generate_lea` を一律に変更してはいけない。実際の LEA 命令は segment base を加えず、memory access 用の実効アドレス計算と区別する。`emit_memory_address` のような別の責務を設け、load/store、RMW、indirect CALL/JMP の pointer load に接続する。CALL/JMP の取得後の target に再度 FS/GS base を足す処理にはしない。

`generate_store32` は現在1 Uopを返す API のため、複数 Uop を必要とする segmented store には emit API または追加 Uop が必要。base-only load/extend の融合も、FS/GS 時の fallback を明示する。

### base と方針の更新

- TLS base の値を各命令の実行時に読む構成なら、base 変更だけで Uop cache を flush する必要はない。A→B→A の切り替えでも同じ cached code を使える。
- base を Uop の即値へ埋め込む構成なら、base 更新時に再 decode、cache key 追加等が必要になるため、初期案にはしない。
- 「全 segment 無視」から「FS/GS 有効」への**方針**変更は生成する Uop 自体を変える。構築時固定が簡単。実行中の切替を提供する場合は flush が必要。
- base/selector の変更はゲスト命令の境界で行う。context 切替は selector と cached base を一緒に保存・復元する。未処理の code-write pending を消してはいけない。

## 4. 単純な base 加算だけでは足りない箇所

| 箇所 | 懸念と必要な処理 |
| --- | --- |
| scalar/RMW/bit-string | signed bit index のアドレス調整、base 加算、bounds を整合させる。offset を完成させ、加算は一度だけ。read と write は同じ linear address を使い、fault 前に flags/register を commit しない |
| POP memory | destination が ESP を使う場合は increment 後の ESP で offset を作ってから base を足す。POP の stack read 自体は SS なので FS/GS override を適用しない |
| PUSH memory / indirect CALL | 明示 operand の読み取りだけを変換し、暗黙の stack write はそのまま |
| string/REP | MOVS/CMPS/LODS の source に override を適用。destination は ES であり FS/GS override の対象にしない。ESI/EDI は offset を保持し、ECX=0 では base 先を検査しない |
| REP bulk kernel | source offset と destination offset の大小で overlap を決めず、変換後の allocation range で判定。logical index wrap、linear address wrap、arena 境界を区別し、partial fault の進捗と完了 span ごとの通知を維持 |
| XLAT | 現在の Uop は segment を持たない。元の FS/GS の区別を lowering/Uop に持たせ、アドレスの規則を保って base を適用 |
| x87 | operand offset と linear address の両方が必要。FDP に linear address を保存してしまわない。selector は元の FS/GS を維持し、store 前の FOP 確保も継続 |
| FS/GS 経由のコード書き換え | 変換後の linear address を既存 `write_memory*` / `write_bytes` / `write_range` へ通す。TLS用の直接 slice write を増やすと invalidation を迂回する |
| selector load | MOV/POP FS/GS、LFS/LGS は descriptor 解決、無効 selector、fault 時の未commitを扱う。selector が同値でも再 load を省略できない |

x87 の非64bit pointer は offset と selector を持つ。現在は base=0 のため同じ値で済んでいる部分が、非ゼロ base で顕在化する。`FpuMinorUop` は address register と source EIP 等を8byteの主 Uop に詰めているため、もう一つ u32 を安易に追加できない。offset を保持する scratch register を使う、operand metadata から成功時に解決する等の設計が必要。[Intel SDM Vol.1 §8.1.8](https://cdrdv2-public.intel.com/819711/253665-sdm-vol-1.pdf)

コード cache の監視は allocation page に対して行われる。従って、正しい linear address で既存 write 経路に合流させれば、FS/GS 経由で通常のコード領域へ書いた場合も現在の全 flush を利用できる。alias ごとの別 cache は不要だが、この合流を省く実装は不可。

## 5. 対応コスト

評価は相対的な実装・検証規模であり、人日や納期の確約ではない。

| 対応範囲 | 規模 | 到達できること／残る制約 |
| --- | --- | --- |
| ホストが設定する FS/GS base、scalar のみ | 小〜中 | 手書き TLS probe は可能。未対応の segmented 命令は明確に拒否し、黙って通常 access にしない |
| 既存命令の FS/GS 対応を揃える | 中 | RMW/bit、stack operand、REP、XLAT、x87 を含めた CPU 側の base 拡張。主な変更は flat の6ファイル前後＋API/テスト |
| Linux/i386 の TLS setup subset | 中〜大 | 上記に TLS descriptor の管理、selector load、`set_thread_area`、必要に応じ `get_thread_area` を追加。単一 thread の小さいアプリを確認できる |
| Windows/x86 TEB を使う subset | 中〜大 | 上記に TEB 配置と必要な ABI。PE loader、API、例外処理等の対応量はアプリ次第 |
| protected-mode segmentation の厳密な互換性 | 大 | present/type/DPL/RPL/limit、null selector、descriptor cache、LDT、fault 分類等。base 拡張と分けて進める |
| 一般 libc・複数 thread・一般 OS アプリ | 大・対象次第 | TLS layout、動的 TLS、loader、clone/CLONE_SETTLS、signal/context、atomics、syscalls/API 等。FS/GS だけの課題ではない |

Linux の `set_thread_area` は単なる GS base setter ではなく、TLS descriptor を設定する API。entry の割当て・検査と guest memory への返却も必要。実装対象外の descriptor 形式を成功扱いで受け入れない。[Linux man-pages](https://man7.org/linux/man-pages/man2/set_thread_area.2.html)

descriptor 登録を行う OS adapter と、selector load で base/属性を解決する CPU 側の役割を分ける。Linux の TLS entry の限定的な表で始める場合、ゲストに完全な GDT/LDT を提供する必要はない。解決を selector load 時に行い、通常メモリアクセスごとの OS callback は導入しない。

既存の `SegmentRegister` に base の格納場所はあるため、base を持つだけの追加メモリは小さい。descriptor table や OS の TLS image は対応範囲に応じて別途必要。

## 6. 実行速度とコードサイズのコスト

### 初期実装

FS/GS 用に少数の flags 非更新 address Uop を設け、既存 load/store に繋ぐ方式を推奨する。アドレス形式次第で追加 dispatch が発生し、現状1 Uopの TLS load が2〜3 Uopになる構成はあり得る。複数 Uop で処理している memory operand なら追加1 Uopで済む場合もある。これは案の静的な dispatch 数であり、実行時間が2〜3倍になるという測定結果ではない。

一般の DS/SS access に segment 判定/base loadを追加する必要はない。ただし新しい primary Uop で巨大な dispatcher の生成コードが変わり、FS/GS を使わない負荷まで影響する可能性はある。従来 HarvardUME は変更しない方針でも、同時コンパイルした場合の native/Wasm の検証は必要。

TLS 用の load/store/RMW を幅・FS/GS別に全て複製する方法は初期案にしない。現在の primary enum はテキスト上220 variant（feature 条件を含む）。追加数、payload の alignment、Minor の入れ子によって8byteの Uopを維持できない場合があり、全 cache のメモリ量と fetch へ影響する。少数の address Uop で始め、hot な TLS:[定数] だけ必要に応じ融合する。

既存の仮想 GPR に FSBase/GSBase のコピーを置く案も考えられるが、`sr_mut()` と二重の状態を同期する必要がある。`ProcessorState` の通常 register access へ分岐を増やす案は避け、segment 状態を一か所から参照する方が簡単。

### 追加の実測が必要なもの

速度低下の％は未測定。少数％を途中の中止条件にはせず、関連処理が揃ってから inline/融合と Wasm payload の受け渡しを見直す。

- FS/GSなしの既存14負荷を native/Wasm、既定 HarvardUME・flat有効 HarvardUME・FlatUME で確認。
- FS/GS:[定数] と register/SIB、read/write/RMW、混在比率、頻繁な base 変更を分けて計測。
- REP の大きい span と destructive overlap、TLS が code と同じ page のケース。
- x87 の offset/linear 分離のコストと8byte Uop構造。
- base 切替前後で cache を温存した場合の correctness と cold decode の分離。

## 7. 推奨する実装順序と判定

1. FlatUME 内のメモリ方針と API を確定する。既定は全 segment 無視を維持し、TLS 用 base を明示的に有効化する。既存の `sr_mut()` に非ゼロ base を入れても無視される挙動を、黙って変更しない。
2. scalar/LEA/indirect branch/RMW と監視への合流を実装し、FS と GS をそれぞれ異なる base で試す。base は動的に読む。
3. POP、bit-string、REP、XLAT、x87 を揃える。未対応形式を通常 flat access へ silently fallback させない。
4. ホスト API で base を設定する小さい probe を用意し、TLS read/write→base更新→同じ cached code の再利用を確認する。
5. OS subset を選ぶ。Linux は `set_thread_area`/descriptor と TCB を足し、freestanding guest 自身による descriptor設定→MOV GS→TLS access を試す。Windows は TEB と必要な ABI を足す。full GDT/LDT や一般 libc 対応は別の到達点にする。
6. 機能が揃った後に性能を評価・最適化する。

主な検証項目は、base=0/非ゼロ、負の TLS offset、32bit wrap、arena外、FS/GS間の違い、flags非更新、同値selector再load、無効selectorの未commit、POPのESP、REP進捗、x87 FDP/FDS、コード自己書き換え、base A→B→A での cache再利用。従来の segment無視モードのテストはそのまま残し、新モードを別に検証する。

## 8. 実際に行った確認

ソースと一次資料を調査し、ホスト GCC 13.3.0 で次を assembly までコンパイルした。エミュレーターで実行した結果ではない。

```c
__thread unsigned tls_value = 7;
unsigned read_tls(void) { return tls_value; }
void add_tls(unsigned value) { tls_value += value; }
```

```sh
gcc -m32 -O2 -ffreestanding -fno-pic -fno-pie -fno-stack-protector -S tls-probe.c
```

read は `movl %gs:tls_value@ntpoff, %eax`、update は `addl %eax, %gs:tls_value@ntpoff` になり、`.tdata` が生成された。典型的な i386 TLS のために GS read だけでなく RMW と TLS relocation/layout も必要になることを確認した。base 対応の試作、実行時間測定、OS subset の拡張はこの調査では行っていない。
