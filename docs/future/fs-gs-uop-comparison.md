# segment base の組み込み方式の比較・概算

2026-10-21。現在の FlatUME を基準とする。segment limit/type/privilege の検査は対象外。
ソース調査と小さい enum layout 実験による設計時の概算であり、base 対応の実装・速度測定は未実施。
速度の数値は予測精度を保証するものではなく、以下の仮定から置いた検討用の幅。

## 比較する方式

| | A: 全メモリ Uop に組み込み | B: FS/GS 付きアクセスを専用 Uop に置換 |
| --- | --- | --- |
| 通常 load/store | segment 属性から base を取得し、offset に加算 | 今の Uop のまま |
| FS/GS load/store | 同じ Uop が base を加算 | segment 属性を持つ専用 load/store が base を加算 |
| RMW | segment を含めてアドレスを解決、一度だけ base を加算 | FS/GS 用アドレス生成＋既存 RMW、または hot な形式だけ融合 |
| 動的な base 更新 | 次の実行から反映 | 同左 |
| DS/ES/SS が非ゼロ | 対応範囲に含められる | 対象外。ゼロを前提とする |

公平な速度比較では DS/ES/SS/CS base=0、FS/GS のみ非ゼロとする。A の方が一般的な機能を持つが、その機能の負担が通常アクセスにもかかる。
「FS/GS 命令」は FS/GS override の付いたメモリアクセスを指す。MOV GS 等の selector load は別処理。
デコーダーは両案とも segment を保持するだけで、方針は Flat lowering が決める。
比較範囲は明示的な operand と暗黙の stack/string access。非ゼロ CS base による命令 fetch/分岐/cache key の変更は含めない。

## 実行時間の概算

A でも segment ごとの分岐は必須ではない。base 配列を索引して無条件加算すればよい。
ただし base が動的な状態なので、DS base が実行時にゼロでも通常は load/add 自体を除去できない。
追加コストには属性の展開、base の取得、アドレス依存の加算、場合によって register pressure/spill がある。
読み書きの arena bounds とコード書き換え監視は両案に共通で、今回増える検査には数えない。

| 負荷 | A | B: 専用の融合 load/store を使う場合 |
| --- | --- | --- |
| FS/GS を使わない通常負荷 | 全体の実行時間 +1〜15% 程度を仮置き | アクセス処理の直接の増分は 0。dispatch/layout 変更による数%の増減は別にあり得る |
| FS/GS が少ない混在負荷 | 同程度の通常アクセスの負担が残る | 例えば全メモリ処理の5%だけなら、base 加算による全体増分は約0.05〜0.75%という計算 |
| メモリアクセスの大半が FS/GS | +1〜15% 程度を仮置き | A に近づく。融合できれば FS/GS の1アクセス当たりの本質的なコストはほぼ同じ |
| 演算主体 / 長い REP bulk copy | 上記より小さい場合がある | 通常経路は維持。REP の base は chunk の開始時に解決し、byte ごとに取得しない |

この幅のモデルは、変更対象のメモリ処理が元の全実行時間の20〜50%を占め、その処理部分が5〜30%重くなる、という仮定。

```text
p = 元の実行時間で、追加処理を受けるメモリ処理部分の割合
c = その部分の相対的な追加コスト
f = その処理時間に占める FS/GS の割合（命令数の割合とは限らない）

A の全体増分 ≈ p × c
B の全体増分 ≈ p × f × c + dispatch/layout による差
例: p=30%, c=20%, f=5% → A +6%, B +0.3% + dispatch/layout 差
```

メモリ待ちが支配的なら隠れる場合があり、逆に小さい load の反復や spill/dispatch の悪化では15%を超える可能性もある。上限の保証ではない。
native と Wasm に同じ範囲を仮置きするが、同じ結果になるという意味ではない。Wasm は属性抽出や JIT の register allocation、native は命令配置等を別に測定する。

### 専用 load/store と追加の address Uop は分けて考える

`LoadSeg32` のように置換すれば load は1 Uopのままで、B が FS/GS 時に A より必ず遅くなるわけではない。
一方、既存の load を再利用するため `ResolveSegment` を追加する方式では、1 Uop のアクセスが2〜3 Uopになる。
後者は**その命令の dispatch 数が2〜3倍**になるが、アプリ全体の時間が2〜3倍になるという意味ではない。
この fallback は FS/GS が少なければ現実的だが、TLS を繰り返す hot loop の第一候補にはしない。

## バイナリサイズと Uop cache

ホストの機械語/Wasm code と、ゲストコードから生成する Uop cache は別に数える。
元のゲスト実行ファイルのサイズは変わらない。

### ホストの実行コード

以下は Flat 側を含む、link 後の到達可能な実行コードの**追加量の粗い予算**。debug 情報、標準ライブラリ、OS adapter、TLS 初期化コードは除く。

| 方式 | 追加量の概算予算 | 根拠・条件 |
| --- | ---: | --- |
| A: 既存 handler を拡張 | 約1〜8 KiB | 約40〜60の経路に属性展開/load/add 等を追加し、lowering と特殊命令を修正。既存 handler 本体は複製しない |
| B: 少数の専用 load/store と共通アドレス生成 | 約4〜24 KiB | 10〜20程度の融合形式と補助処理を追加。bounds/書き換え処理の inline 複製量で変わる |
| B: 全メモリ形式をそのまま複製 | 上記より大きくなる恐れがあり、別見積もり | 44種類前後を複製し、dispatch の表現が変わり得る。推奨しない |

この KiB 値は compiler 出力を測った差分ではない。各経路の追加を数十〜百数十 byte、補助処理と lowering を残りの予算と見た桁の見積もりで、inline/LTO/JITによって範囲外にもなる。
既存の計測資料には Flat frequency の Wasm code section が約300 KiBという例がある。その規模を仮の分母にすれば A は約0.3〜2.7%、B は約1.3〜8%の増加予算。現在 HEAD の実測値や最終 module サイズの予測ではない。
native の実行ファイル全体に対する割合は、strip/debug/他機能で分母が変わるためここでは置かない。
native と Wasm それぞれの実際の追加量は、同一 feature/build 条件で測る必要がある。

### Uop の8byte維持が重要

現在の主 Uop は feature 条件を含めテキスト上220種類。そのうち scalar memory/RMW/POP memory の抽出対象は44種類。
FS/GS を共通の segment 属性にしても44種類を丸ごと複製すると264種類、FS/GSごとの複製ならさらに増える。
256を超える構成は discriminator の表現が変わり得るため、8byte維持を実際に検証する必要がある。

小さい Rust enum 模型を現在の rustc でコンパイルすると、220 variant＋7byte payload は8byte、264 variantでは12byteになった。
これは実際の Flat Uop の変更後の測定ではない。一方、模型の load payload に segment を1項目追加しただけでは8byteを維持できた。
従って「segment 属性を加えたら必ず12byte」でも「256を超えたら必ず12byte」でもない。
register/segment の bit packing と Rust の enum layout に依存する。現在のテストの `size_of::<Uop>() == 8` を両 target で維持する。

| 設計 | 生成済み Uop cache の増分 |
| --- | --- |
| A: 属性を詰め、8byte・Uop数を維持 | 本体の増分は0。補助 metadata の追加は別途 |
| B: 8byte の専用 Uop へ1対1置換 | 同上 |
| B: FS/GS 付きアクセスに1 Uop追加 | その箇所あたり8byte追加。全 Uop数に対する追加数の比だけ増加 |
| 主 Uop全体が8→12byteに拡大 | FS/GSの有無にかかわらず全 Uop本体が50%増加 |

例えば10万 Uopの trace は約781 KiB。1対1置換なら変わらず、1,000箇所に1 Uop追加なら約7.8 KiB増、12byte化すると約391 KiB増。
cache の増加率は静的なコード中の構成比、実行時間の増加率は動的な実行頻度で決まるため、同じ割合ではない。

## その他の懸念

- A は通常の stack、CALL/RET、string destination にも SS/ES base を適用する必要がある。明示 operand だけ直しても「全メモリアクセス対応」にはならない。非ゼロ CS の命令 fetch はさらに別の対応。
- B は DS/ES/SS=0 という契約を明示する。将来それらも必要になれば専用 base アドレス生成を一般化できるが、現在の通常 Uop をそのまま使う経路とは区別する。
- 両案とも segment base 加算は一度だけ行う。既に線形アドレスになった RMW scratch や公開メモリ API に再加算しない。
- LEA は offset の計算のまま。POP の destination は ESP 更新後、REP の source/destination は各自の segment を使う。x87 はアクセス用の linear address と保存用 offset/selector を分離する。
- limit を無視しても、arena bounds、32bit wrap、fault 時の未commit、コード書き換え通知は残る。segment 越しに同じコードへ書く場合も線形アドレスで既存通知へ合流させる。
- 動的な base を読む限り base 更新で cache flush は不要。base の定数化やホストポインター化を行うと、更新・arena再配置時の失効処理が必要になる。
- A の zero-base 特殊化を lowering で行えば通常アクセスの負担を減らせるが、base の可変性を制限するか更新時に再生成が必要になる。通常経路を専用化する点では B に近い設計になる。
- B は主 dispatcher の追加で既存経路の生成コードにも影響し得る。直接の追加処理が0でも速度不変を保証しない。通常負荷の数%の差だけで途中の実装を棄却せず、機能完成後に融合・inline・配置を見直す。

## 推奨

目的が TLS/TEB と既存 Flat 性能の維持なら B。FS/GSを共通属性にした専用の融合 load/store を少数追加し、RMW/rare形式は共通の segment address helper を使う。全形式・FS/GS別の機械的な複製はしない。
DS/ES/SSも非ゼロで使うことが要件なら A を選ぶ理由がある。その場合も runtime 判定や間接 callback を各アクセスへ増やさず、8byte/Uop数を保つ。
将来の A を妨げないよう、B の base 解決 helper 自体は segment index を受け取る構造にできる。

次の検証では通常負荷と FS/GS比率0/1/5/25/100%、read/store/RMW、base切替、REP/x87を分け、native/Wasmそれぞれで時間・text/code section・Uopサイズ/個数を比較する。

参照: [先行調査](fs-gs-support-analysis.md)、[現在の Uop](../../plum/src/flat/uop.rs)、[lowering](../../plum/src/flat/tracer.rs)、[実行](../../plum/src/flat/mod.rs)、[既存のサイズ計測](../DESIGN.md)。
