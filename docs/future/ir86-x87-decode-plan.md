# IR86 x87初期デコード移行計画

作成日: 2026-10-17。状態: 計画のみ、未実装。

対象: `ir86`、`plum`、`tools/disasm`。関連: [x87の現状（DESIGN.md 9.1節）](../DESIGN.md)。本計画は従来の「ESCをPLUM側で個別命令へ分類する」という暫定方針を置き換える。

## 1. 決定事項

- IR86にCargo feature `x87`を追加し、デフォルトは無効とする。
- 無効時のD8〜DFは、従来の`ESC_Rb` / `ESC_MbA16` / `ESC_MbA32`のまま返す。命令長、operand/address-size、segment、opcode/ModRM情報の扱いを維持する。
- 有効時はIR86でx87の個別命令とoperand形式を確定する。PLUMはIRからUopへの変換を担当し、opcode/ModRMによる命令再分類を行わない。
- PLUMの`x87`は`ir86/x87`も有効にする。逆方向には連動させない。IR86だけで個別命令のデコード・表示・テストができるようにする。
- FPU専用`FpuMinorUop`、親Uopの8byte上限、演算backend、生値保持、数値制約、実行時faultの規則は維持する。
- FXSAVE/FXRSTOR、SSE/MMX、数値精度の改善、A16での実行対応、旧式・非公開命令の追加はスコープ外。

主目的は、命令解釈をIR86へ集約し、IR・逆アセンブル・テストから命令名とoperand形式を読めるようにすること。実行性能の向上は採用理由にしない。

## 2. 現状と変更後の経路

現在:

```text
バイト列 → IR86: ESC + opcode/ModRM/EA
        → PLUM: opcode/ModRMを分類 → FpuMinorUop → FPU実行
```

変更後:

```text
ir86/x87なし: バイト列 → ESC → PLUMではFPU無効エラー
ir86/x87あり: バイト列 → 個別x87 IR → IRからFpuMinorUopへ変換 → FPU実行
                                   └ PLUM/x87なしならFPU無効エラー
```

IRは現在も一命令ずつ変換され、常駐キャッシュに保存されない。キャッシュするのはUopである。この性質を維持する。

## 3. featureと依存関係

予定するCargo設定:

```toml
# ir86/Cargo.toml
[features]
default = []
x87 = []

# plum/Cargo.toml（既存featuresへ追加）
[dependencies]
ir86 = { path = "../ir86", default-features = false }

[features]
default = []
x87 = ["dep:libm", "ir86/x87"]

# tools/disasm/Cargo.toml
[dependencies]
ir86 = { path = "../../ir86", default-features = false }

[features]
default = []
x87 = ["ir86/x87"]
```

IR86は`no_std`のままとし、libmやPLUMへの依存を追加しない。`ir86/x87`は構文の認識を有効にするfeatureであり、数値演算の対応やCPUIDの能力を意味しない。

### feature統合への対応

Cargoは同一IR86のfeaturesを統合するため、次の全組合せを扱う。

| PLUM/x87 | 実際のIR86/x87 | デコード結果 | PLUMの動作 |
|---|---|---|---|
| 無効 | 無効 | ESC | 実行位置でFPU無効エラー |
| 無効 | 有効 | 個別IR、一部ESC fallback | 実行位置でFPU無効エラー |
| 有効 | 有効 | 個別IR、一部ESC fallback | 既存FPU実行へ接続 |
| 有効 | 無効 | feature依存で発生しない | — |

`ir86/x87`を別の利用者が有効にしても、PLUMのFPU状態やlibmを有効にしない。

IR86に常時利用可能な`IrOp::is_x87()`を追加する。個別命令のmatch armはIR86側のcfgで生成し、ESCとWAITも分類に含める。PLUM/x87無効時はこの分類で個別IRもESCも`FpuDisabled`へ落とす。依存先のfeature状態をPLUM側の`#[cfg(feature = "x87")]`で推測しない。

## 4. IRの表現

既存の整数IRと同様に、`IrOp`へ命令名・operand形式ごとのvariantsを追加する。PLUMの`FpuOp`やbit-packedな`arg`をそのままIRへ持ち上げない。

概念例（最終的な綴りは既存の命名規則に合わせる）:

```rust
#[cfg(feature = "x87")]
FLD_M32real_A32(MemOpr32),
#[cfg(feature = "x87")]
FILD_M64int_A32(MemOpr32),
#[cfg(feature = "x87")]
FADD_ST0_STi(X87Reg),
#[cfg(feature = "x87")]
FADD_STi_ST0(X87Reg),
#[cfg(feature = "x87")]
FADDP_STi_ST0(X87Reg),
#[cfg(feature = "x87")]
FNSTENV_A32(MemOpr32, X87EnvFormat),
```

- ST(0)..ST(7)を表す小さな`X87Reg`を定義する。整数GPRのreg8型で代用しない。
- 32/64/80bit real、16/32/64bit integer、10byte BCDを区別する。同じメモリ幅でも整数とrealを混同しない。
- A16/A32は別variantとしてデコード可能にする。PLUMがA16を実行できるという意味にはしない。
- ST0/STi、STi/ST0、逆順、pop、pop数、FCMOV条件、FCOM/FUCOM、整数EFLAGS更新の有無をIRから判別できるようにする。
- 環境命令に`X87EnvFormat`を持たせ、operand-sizeによる14/28byte環境・94/108byte全状態を区別する。アドレス幅や通常の数値operand幅とは独立させる。
- `mnemonic()`はFSUBR、FISTTP等を返し、`name()`はoperand形式まで区別する。`n_args()`とDebug表示も整合させる。
- IRにruntimeのFPU値、CW、PC/RC、タグ、UME専用レジスタ、実行対応フラグを埋め込まない。
- runtimeの丸め制約や数値の定義域を理由にデコードを拒否しない。既存の実行時検証へ残す。

### WAIT、未知のencoding、FOP

- 9Bは独立した`IrOp::WAIT`のままとし、後続のFN命令へ融合しない。WAIT+FNINITなどの命令位置と命令長を維持する。
- ESCの3variantsと`EscOpcode`はfeature有効時も残す。未分類・予約・今回対象外のencodingは情報を失わずESCへfallbackする。
- PLUM/x87有効時のESC fallbackは再分類せず、実行位置で`Unimplemented`へ落とす。トレース先読み時にpanicや早期終了させない。
- x87のFOPには元のencodingが必要だが、正規化IRからencodingを逆算しない。既存のimmutable codeとsource EIPを使った環境保存時の遅延復号を維持する。

## 5. IR生成器

変更候補: `ir86/src/ir.csv`、`ir86/build.rs`、生成される`ir86/src/_generated/ir.rs`。

- CSVへ省略可能なfeature列を追加する案を基本とする。既存3列はそのまま解釈し、新しいx87行だけ`x87`を指定する。
- feature情報を`IrSource`から生成後の各variantへ伝播する。必要なx87 operand形式の生成モードを追加する。
- `IrOp`、`Mnemonic`、`mnemonic()`、`name()`、`n_args()`、`is_x87()`の関連定義・match armに一貫して`#[cfg(feature = "x87")]`を付ける。
- 同じmnemonicに複数行がある場合はfeature条件を集約し、未定義のMnemonic参照やcfgによる重複定義を防ぐ。
- build.rs実行時の`CARGO_FEATURE_X87`で生成内容を出し分けない。常にcfg付きの同一ソースを生成する。
- featureなし→あり→なし、別target、並行ビルドで生成ファイルの内容が変化しないことを確認する。同一内容なら再書込みを省き、更新時は途中の内容を読まれない書込み方式にする。生成先をOUT_DIRへ移す改修は今回の必須にしない。

## 6. デコーダーとUop変換

### IR86

`ir86/src/decoder.rs`のD8〜DF分岐から、cfg付きの専用モジュール（例: `ir86/src/x87.rs`）へ個別分類を委譲する。

- ModRM、SIB、displacement、segment、66/67、use16/use32の既存処理を再利用する。
- opcode/ModRMで命令が一意になる段階で個別IRを構築し、同じ命令をPLUMで再判定しない。
- featureなしでは既存ESCの構築だけを行い、詳細分類モジュール・型・表示コードを含めない。
- 不足バイトでのfetchエラー、命令消費長、次命令位置を維持する。prefixの扱いをこの移行で無断変更しない。

### PLUM

`plum/src/ume/tracer.rs`にcfg付きのx87 loweringを接続する。必要なら専用ファイルへ分ける。

- 個別IRの命令・operandから、既存と同じFPU Uopの操作・引数・EA・segment・source EIPを生成する。
- EAは実行時に現在のGPRから計算する既存方式を維持する。トレース生成時にメモリ値を取り込まない。
- `plum/src/fpu/uop.rs`のopcode/ModRM分類表を削除し、IRから利用する意味単位のコンストラクターへ置き換える。
- A16は既存同様`FpuAddress16`へ落とす。featureなしでは個別IRも`FpuDisabled`へ落とす。
- 演算dispatcher、FPU状態、数値backend、例外・メモリcommit規則は移行に伴って変更しない。
- `FpuMinorUop`の7byte、親Uopの8byte上限と、命令当たりのUop数を維持する。

## 7. 移行手順

| 段階 | 作業 | 完了条件 |
|---|---|---|
| P0 基準固定 | 現行の全x87 encoding→意味/Uop変換、feature別ビルド・型サイズ・ベンチ結果を保存 | opcode/形式の対応表と比較用vectorがある |
| P1 feature・生成器 | IR86/disasm feature、PLUMからの連動、cfg付き生成、operand型、`is_x87()` | 全feature組合せで生成・ビルドが安定する |
| P2 個別IR | 全実装済みx87命令の分類、A16/A32・環境形式、表示情報を追加 | IR86単体で意味・型・命令長のテストが通る |
| P3 Uop移行 | loweringを接続、旧分類表を削除、ESC fallbackを整理 | 既存と同じUop列・実行結果・fault位置を維持する |
| P4 検証・文書 | feature統合、逆アセンブル、性能/サイズ比較、README更新 | 下記受入条件を満たし、測定差と残る制限を記録する |

比較用の旧分類表はテストfixtureとして移行中だけ利用してよい。本番に2種類のopcode分類表を残さず、最終的なテストは独立した期待IR/operandのvectorにする。

## 8. 検証

### デコードと実行

- featureなし: ESCのvariant、metadata、命令長、既存整数命令の結果が従来と一致する。
- featureあり: D8〜DF × ModRMの全組合せを分類し、既知の個別命令とESC fallbackを明示的に区別する。メモリ形式はdisplacement等を補って全アドレス形式を検証する。
- ST方向/pop、FSUBR/FDIVR、整数/realの区別、FCMOV全条件、FCOMI系、FNSTSW AX、BCD、剰余、指数対数を対象にする。
- use16/use32 × 66/67 × segment override、環境の14/28・94/108byte、fetch末尾の不足バイト、次命令位置を確認する。
- WAIT単独、WAIT+FN命令、未知のencoding、先行命令の実行後に拒否されることを確認する。
- 既存のFPU専用42テストと実機oracle、全workspaceテストを継続する。数値結果を変えず、cached trace再利用、メモリfault時の状態保持、再試行、EIP/FOP/pointerも維持する。
- 旧/newのUop比較は操作・operand・EA・segment・source EIPを明示的に比較する。enumのpaddingを含むメモリmemcmpは使わない。
- disasmのmnemonic/IR統計が、有効時はFLD/FADD等、無効時はESCになることを確認する。

### feature matrix

実装後に実行するコマンド例:

```sh
cargo test -p ir86 --no-default-features --offline
cargo test -p ir86 --features x87 --offline
cargo test -p plum --no-default-features --offline
cargo test -p plum --features x87 --offline
# IR86だけが有効になるfeature統合ケース
cargo test -p plum --no-default-features --features ir86/x87 --offline
cargo test --workspace --all-features --offline
cargo check -p disasm --no-default-features --offline
cargo check -p disasm --features x87 --offline
cargo check -p ir86 --target wasm32-unknown-unknown --no-default-features --offline
cargo check -p ir86 --target wasm32-unknown-unknown --features x87 --offline
cargo check -p plum --target wasm32-unknown-unknown --no-default-features --offline
cargo check -p plum --target wasm32-unknown-unknown --features x87 --offline
```

`--no-default-features`だけでは依存先featuresが無効になる保証はないため、統合ケースを別に試す。IR86有効・PLUM無効でもFPU状態/libmが含まれないことを依存グラフとビルド成果物で確認する。

## 9. 速度・バイナリ・メモリの受入条件

現状の簡易測定: `IrOp`は845 variants・16byte、alignment 4。既存と同程度のoperandを持つ200 variantsを足した仮定義も、native/wasm32で16byteだった。これは実装後の保証ではなく、設計の根拠とする。

| 項目 | 検証・基準 |
|---|---|
| IRサイズ | feature別・native/wasm32で`size_of::<IrOp>() == 16`を検証。増える場合はoperand表現を見直す |
| Uop/常駐メモリ | 親Uopは8byte以下、同じ入力でUop数・EA生成数を維持。IRの永続キャッシュを追加しない |
| warm実行 | 生成済みトレースで比較。中央値が3%以上悪化した場合を要調査の目安とし、分散と再現性を併記する |
| cold変換 | IR単独decodeと初回Uop生成を分け、命令数当たり時間を測る。UME生成・allocationを含む時間も別に示す |
| バイナリ | 同一toolchain/最適化/LTOでnativeのtext/rodata、strip後の実行ファイル、wasm成果物を比較。featureなし/あり、disasmを別々に報告 |
| featureなし | 詳細デコーダー・x87専用IR型/表示がcfgで除かれることを確認。分類の常時組込みを避ける |
| ソース | 手書きコードと生成コードの行数を分ける。opcode分類の重複がなくなることを確認 |

固定vectorは整数のみ、x87の軽い算術中心、メモリ形式中心、三角関数等の重い演算を用意する。warm計測に初回変換やprofile文字列生成を混ぜない。cold比率が小さいケースで総実行時間だけを見てデコード性能を評価しない。

前回の「数KiB〜十数KiB」というバイナリ増加の見積もりは、IRの詳細デコードを常時含める場合の未測定の仮置きだった。本計画ではfeatureなしの詳細分類を除外できるため、その数値を流用しない。実装後のA/B測定で確定する。

## 10. 最終成果物

- デフォルトESC / feature有効時の個別IRというAPIと、feature統合時の動作をREADMEに記載する。
- 命令名・operand形式をIR86の定義とテストから追える状態にする。IRの構文認識と、PLUMの実行対応・数値制限は分けて説明する。
- ベンチとサイズ比較、feature matrix、残る制限を記録する。
- FXSAVE/FXRSTORを未実装x87命令として再び残作業へ含めない。
