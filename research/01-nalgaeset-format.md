# 날개셋 (Nalgaeset) "입력 설정" XML — Format Specification

Reverse-engineered reference for building a faithful generic interpreter in Rust.

Source config analysed: `/home/nemo/git/lens/provision/config/nalgaeset.xml`
(root `<EditContextSetting version="0x500">`, three `InputEntry` items; entry #0 is the
세벌식-맞춤 + CNgsImeEx generator that exercises every feature of interest).

## Authoritative sources

날개셋 한글 입력기 is freeware by 김용묵 (Kim Yong-mook). It is **not** open
source, so there is no canonical source tree to cite; the deep semantics below are derived
from (a) the official site, (b) the bundled Korean help manual as quoted by the author and
community, and (c) direct decoding of the XML data. Confidence is marked per claim.

- 날개셋 — 전반적인 특징 (official): <http://moogi.new21.org/ngs_menu1.htm> — confirms the
  expression language follows **C-language operators**, that key values are *expressions* not
  fixed chars, gives the canonical worked example `T ? H3|O : /` ("한글 조합 중이고[T!=0]
  중성이 전혀 입력되지 않았을 때[E==0]는 세벌식 중성 ㅗ(H3|O)를 입력"), and shows 한자 변환 =
  `C0|0x8?` in 단축글쇠.
- 날개셋 — 고급 활용 기능 (official): <http://moogi.new21.org/ngs_menu2.htm> — 모아치기 /
  미완성 한글 / 세벌식 초·중·종성 자유 입력 conceptual background; explains why the automata
  has special states.
- 날개셋 — 구현체 소개 (official): <http://moogi.new21.org/ngs_imple.htm> — editor vs IME,
  옛한글, 채움문자→완성자 conversion.
- 날개셋 — English survival guide: <http://moogi.new21.org/en/ngs/index.htm> — confirms the
  layer model: **Editor Layer** (Shortcut Keys live here) over per-entry **input scheme**
  (글쇠 인식) + **character generator** (조합 생성), each rated 빈/기본/고급 (Void/Basic/
  Advanced) — exactly the `object="CInputScheme|CBasicInputScheme|CAdvancedScheme"` and
  `object="CIme|CNgsIme|CNgsImeEx"` triples in the XML.
- 신세벌식 P 기호확장 forum thread: <https://bbs.pat.im/viewtopic.php?t=932> — references the
  T variable carrying composition state ("T의 3가지 상태값", "한글 조합 이외의 조합 상태도 T
  변수에 넣기") and P as the modifier/shift state.
- .ist/.key loading overview: <https://pat.im/949> — ".ist 파일에는 글쇠 배열, 글쇠 인식 옵션,
  입력 일반, 낱자 처리, 오토마타 설정값까지 들어간다"; ".key 파일에는 47개 글쇠 값."
- ko.wikipedia / namu.wiki 날개셋 — general framing; namu cites 김용묵's thesis "한글
  입력·편집기의 통합적 설계와 구현에 관한 연구."

This XML is the **종합 설정 (comprehensive setting, `.set`)** file (root `<EditContextSetting>`)
serialised as text. The three settings file types are nested sub-roots of one schema:
`.set` (종합 설정, `EditContextSetting`) ⊃ `.ist` (유형 / input type, `InputEntry`) ⊃
`.key` (글쇠배열 / key layout, `KeyTable`). The author states (ngs_menu1) settings can be saved
"바이너리뿐만 아니라 …XML 방식으로도" and edited in a text editor.

---

## 0. Document / layer structure (recap)

```
EditContextSetting version=0x500
└─ EditorLayer flag=DEL_MOVE|ARROW_MOVE        # global editing layer
   ├─ ShortcutTable        # global keys recognised before any input entry (한/영, 한자, Caps)
   └─ FinalConvTable       # conjoining/old-hangul jamo → compatibility jamo, for output of standalone jamo
└─ InputLayer default=0 current=2              # which entry is default / currently active
   └─ InputEntry × N        # a "자판/입력 항목" = scheme + generator
      ├─ InputSchemeSetting object=CInputScheme|CBasicInputScheme|CAdvancedScheme  (빈/기본/고급)
      │  └─ KeyTable          # raw key (0x21..0x7E) → value-expression
      │  └─ KeyProgramTable   # (advanced) down/up timing programs; empty here
      └─ GeneratorSetting object=CIme|CNgsIme|CNgsImeEx (빈/기본/고급)
         ├─ UnitMixTable      # 낱자 결합 규칙 (jamo a + jamo b → jamo z, per category)
         ├─ VirtualUnitTable  # virtual unit id → real unit (used by automata/keys)
         ├─ UnitAbbrTable     # 낱자 약자 (empty here)
         ├─ AutomataTable     # the composition state machine
         ├─ Extra/Bksp        # backspace-deletion granularity rules
         ├─ UserCompoTable / UserCandiTable / UserKeyTable
         ├─ HanSubstTable / UnitSubstTable
```

`InputLayer default="0" current="2"`: entry 0 (세벌식) is the boot default; entry 2 (the empty
CInputScheme/CIme = pass-through Latin/QWERTY) is currently selected. Entries are switched by
the `!A`/`!B`… shortcut values (see §6).

---

## 1. KeyTable value-expression language

`<KeyTable name=… flag=… from=33 to=126>` maps each **base ASCII keycode** `at="0xNN"`
(0x21..0x7E = printable ASCII, i.e. the *unshifted* US-QWERTY character the OS reports) to a
value-**expression**. `from`/`to` give the populated keycode range. The expression is
evaluated at key-down to decide what the key produces.

> Official: "글쇠배열은 …수식으로 표현되며 …수식은 C언어 연산자의 문법을 따릅니다."
> (ngs_menu1) — so operator set/precedence = C. Result is an `int`.

### 1.1 Grammar (EBNF)

```
expr      := ternary
ternary   := orexpr ( "?" expr ":" expr )?          # C ternary, right-assoc
orexpr    := andexpr ( "||" andexpr )*
andexpr   := bitor   ( "&&" bitor   )*
bitor     := bitxor  ( "|"  bitxor  )*               # NOTE: see §1.3 — bare "|" after a tag prefix is the tag separator, NOT bitwise-or
bitxor    := bitand  ( "^"  bitand  )*
bitand    := equality( "&"  equality)*
equality  := rel      ( ("=="|"!=") rel )*
rel       := shift    ( ("<"|">"|"<="|">=") shift )*
shift     := add      ( ("<<"|">>") add )*
add       := mul      ( ("+"|"-") mul )*
mul       := unary    ( ("*"|"/"|"%") unary )*
unary     := ("!"|"-"|"~")? primary
primary   := number | ident | tagged | "(" expr ")"
tagged    := TAG "|" operand                          # H3|… , C0|…  (and !A is its own form)
number    := "0x" HEX+ | DEC+
ident     := "T" | "P" | "A".."E" | … (context variables, §3 for automata)
TAG       := "H3" | "C0" | …
```

C operator precedence applies (ternary lowest, then `||`, `&&`, `|`, `^`, `&`, `==`/`!=`,
relational, shift, additive, multiplicative, unary). This matters for the arithmetic forms
(§1.4).

### 1.2 Primary value forms observed

| Form | Meaning | Example in file |
|---|---|---|
| `0xNN` / decimal | literal Unicode scalar / number to emit verbatim | `0xB7` (·), `0x2026` (…), `0x21` (!) |
| `H3\|<unit>` | **Hangul낱자 reference** — emit/feed a jamo "unit" into the composer | `H3\|_GG`, `H3\|O_`, `H3\|0x810000` |
| `C0\|<n>` | **control / 특수글쇠 (special-key) command** code `n` | `C0\|0xA`, `C0\|2`, `C0\|0x82` |
| `T ? A : B` | C ternary on context vars | `T ? H3\|_J : 0x23` |
| arithmetic | computed code (case toggling etc.) | `119^(P&1)<<5` |
| `!A` | "switch to input entry A" pseudo-value (shortcuts) | `value="!A"` |

### 1.3 The tag prefixes `H3|` and `C0|`

The leading `XX|` is a **2-hex-digit type tag** on the value, *not* a bitwise OR at the top
level. Internally a 날개셋 "nchar" (the value a key returns, per the `ngs_nchardlg` dialog in
ngs_menu1: "한글, 비한글 일반 문자, 또는 다양한 지시를 내리는 비문자") is a tagged 32-bit
value where the high byte selects the kind:

- **`H3|` = Hangul낱자 (jamo) value.** `H3` ≈ "Hangul, 3-set/낱자 form". The operand is a unit
  code (mnemonic like `_GG`, `O_`, or a raw `0x……` jamo encoding). When such a key is pressed
  the unit is **fed into the syllable composer / automata** (not emitted as a finished char).
  This is the official `H3|O` = 세벌식 중성 ㅗ notation from ngs_menu1.
- **`C0|` = control / command (특수글쇠) value.** Operand is a command id. `C0|0x82` (= 한자
  변환, confirmed official "한자 변환(C0|0x8?)"), `C0|0xA`, `C0|0xC`, `C0|0xE`, `C0|0xF`,
  `C0|2`. These trigger 특수글쇠 behaviours (cursor move, delete-component, syllable-boundary,
  Hanja, etc.) rather than producing a glyph. (See §1.6 for the specific codes in this file.)

> Confidence: HIGH for the *kind* of each tag (H3=jamo, C0=command) — `H3|O` is the literal
> example in the official page and `C0|0x82`=Hanja is officially stated. The exact internal
> bit layout (which byte is the tag) is a reasonable hypothesis (MEDIUM); an interpreter only
> needs to treat the tag as an enum discriminant.

Other tags exist in the format family (e.g. plain literal chars carry an implicit "character"
tag); only `H3|` and `C0|` appear in this config besides bare literals.

### 1.4 Arithmetic forms — case toggling (the Dvorak table)

Entry #1 (로마자 드보락, CAdvancedScheme) is pure Latin remapping and shows the arithmetic
idiom:

```
119^(P&1)<<5      # at 0x2C  (',' key position) → letter w / W
118^(P&1)<<5      # at 0x2E  → v / V
65 ^(P&1)<<5      # at 0x41  → A / a
```

Decoding with C precedence (`<<` binds tighter than `^`):

```
value = 119 ^ ((P & 1) << 5)
      = 'w'(0x77) ^ (shift ? 0x20 : 0)
```

- `P` = **modifier/shift state** of the key event. `P & 1` = Shift pressed (1) or not (0).
- `(P&1)<<5` = `0x20` when shifted, else `0`.
- XOR with `0x20` flips ASCII bit 5, i.e. toggles **letter case** (0x77 'w' ↔ 0x57 'W').

So `NNN^(P&1)<<5` = "emit the letter whose **lowercase** code is `NNN`, upper-cased when
Shift is held." The literal codes in the table are the *Dvorak* letter at that QWERTY
position (e.g. QWERTY `,` → Dvorak `w`). Keys with no case (digits, symbols) use plain
literals.

> Your hypothesis is CONFIRMED: **P encodes modifier state, P&1 = Shift, `^(P&1)<<5` toggles
> the 0x20 ASCII case bit.** (Derived from the data; consistent with official statement that
> case/Numlock-dependent output is expression-driven, ngs_menu1: "Capslock 및 Numlock의 점등
> 여부에 따른 입력 문자 구분".) Confidence HIGH.

Note `P` is a **bitfield**: bit0 = Shift is the only one exercised here, but the format allows
higher bits (Ctrl/Alt/Caps/Num lamp etc.) — treat `P` as an opaque modifier bitmask and only
bit0 is load-bearing in this config.

### 1.5 The `T` variable and ternaries (the 세벌식 table)

Entry #0 keys use `T` heavily:

```
0x23  T ? H3|_J  : 0x23     # if composing: jongseong ㅈ(_J); else literal '#'
0x24  T ? H3|0x1F4 : 0x24   # if composing: a specific jamo (raw 0x1F4); else '$'
0x3C  T ? C0|0xF : 0x5B     # if composing: special-key 0xF; else '['
0x3E  T ? C0|0xC : 0x5D     # if composing: special-key 0xC; else ']'
0x40  T ? H3|_RG : 0x40     # if composing: jongseong ㄺ(_RG); else '@'
0x60  T ? C0|0xE : 0x2A     # if composing: special-key 0xE; else '*'
```

- **`T` = the automaton/composition state value** (the current `Automata state` id, see §3),
  used as a boolean: `T != 0` ⇒ "currently composing Hangul." This is the official meaning:
  "오토마타 상태를 나타내는 T의 값이 0이 아니고" (ngs_menu1). The forum notes T can also carry
  non-Hangul composition states when the option "한글 조합 이외의 조합 상태도 T 변수에 넣기" is
  on, and that T's three composition states correspond to ids reported as 101–103 in some
  contexts (an offset display); for this config the raw automata ids are 0/1/2 (§3).
- `E` (used in the official `T && E==0` example, though not in *this* file's key exprs) =
  "중성이 입력되었는가" within the current syllable. (Same `E` appears in the automata, §3.)
- The pattern "produce a jongseong/special-key while composing, else a punctuation symbol" is
  the classic 공병우 세벌식 trick where shifted/number-row keys double as 받침 only mid-syllable.

> Your hypothesis is CONFIRMED: **T is the composition/automaton state predicate (T≠0 = mid-
> composition).** Confidence HIGH (official).

### 1.6 `C0|` command codes seen in this config (entry #0)

These are 특수글쇠/제어 ids. Exact numeric meanings are not published; mapping inferred from
the documented 세벌식 특수글쇠 set (ngs_menu2 lists: 낱자 단위 삭제, 글자 단위 이동+조합 재현,
종성을 다음 글자로 이동, 음절 경계, 한자 변환). Treat as an enum to be confirmed empirically.

| Value | Key (at) | Hypothesised meaning | Confidence |
|---|---|---|---|
| `C0\|0xA` | 0x25 `%` | likely 음절 경계 강제 / 조합 분리 (start new syllable) | LOW |
| `C0\|2` | 0x54 `T` | a 특수글쇠 (component delete or boundary) | LOW |
| `C0\|0xC` | 0x3E (`T?…`) | special-key, mid-composition only | LOW |
| `C0\|0xE` | 0x60 (`T?…`) | special-key, mid-composition only | LOW |
| `C0\|0xF` | 0x3C (`T?…`) | special-key, mid-composition only | LOW |
| `C0\|0x82` | shortcut VK_HANJA | **한자 변환** | HIGH (official `C0\|0x8?`) |

For interpreter purposes: parse `C0|n` to `Command(n)`; only `0x82`=Hanja is needed for basic
correctness, the others can be stubbed/logged until verified against the running IME.

---

## 2. Hangul unit (낱자) mnemonics

`H3|<unit>` operand is either a **mnemonic** or a **raw numeric jamo code**. Mnemonics follow a
consistent scheme tied to *position*:

- **Choseong (초성):** mnemonic ends in `_` → `G_ N_ D_ R_ M_ B_ S_ J_ C_ K_ T_ P_ H_` plus
  doubles `GG DD BB SS JJ` (written without trailing `_` in UnitMix), and `Q_` (=ㅇ, "이응"),
  `O_`/`U_` are *jungseong*, careful: trailing `_` = choseong only for consonants. (ㅇ choseong
  is `Q_`.)
- **Jungseong (중성):** **bare** vowel mnemonic, no underscore: `YO YU YA YE YEO AE EO EUI YAE`,
  and the simple vowels are written with trailing `_`: `O_ U_ A_ I_ E_` plus `EU` (ㅡ). Compound
  vowels appear only in UnitMix `to=`: `WA WAE OI UEO WE WI`.
- **Jongseong (종성/받침):** mnemonic **starts** with `_` → `_G _GG _GS _N _NJ _NH _D _R _RG
  _RM _RB _RS _RT _RP _RH _M _B _BS _S _SS _NG(=_Q?) _J _C _K _T _P _H`. (`_S`=ㅅ받침,
  `_SS`=ㅆ받침, etc.)

### 2.1 Mnemonic → jamo table

Letters map to the standard Hangul romanization of the jamo. **CHO** = leading consonant,
**JUNG** = vowel, **JONG** = trailing consonant. Unicode columns: "compat" = U+31xx
compatibility jamo (what 채움 conversion targets, §4); "conj" = U+11xx conjoining jamo
(L=choseong 0x1100+, V=jungseong 0x1161+, T=jongseong 0x11A7+).

Choseong (used in this file: `G_ N_ D_ R_ M_ B_ S_ J_ C_ K_ T_ P_ H_ Q_` + doubles):

| Mnem | Jamo | compat | conjoining L |
|---|---|---|---|
| `G_` | ㄱ | U+3131 | U+1100 |
| `GG` | ㄲ | U+3132 | U+1101 |
| `N_` | ㄴ | U+3134 | U+1102 |
| `D_` | ㄷ | U+3137 | U+1103 |
| `DD` | ㄸ | U+3138 | U+1104 |
| `R_` | ㄹ | U+3139 | U+1105 |
| `M_` | ㅁ | U+3141 | U+1106 |
| `B_` | ㅂ | U+3142 | U+1107 |
| `BB` | ㅃ | U+3143 | U+1108 |
| `S_` | ㅅ | U+3145 | U+1109 |
| `SS` | ㅆ | U+3146 | U+110A |
| `Q_` | ㅇ | U+3147 | U+110B |
| `J_` | ㅈ | U+3148 | U+110C |
| `JJ` | ㅉ | U+3149 | U+110D |
| `C_` | ㅊ | U+314A | U+110E |
| `K_` | ㅋ | U+314B | U+110F |
| `T_` | ㅌ | U+314C | U+1110 |
| `P_` | ㅍ | U+314D | U+1111 |
| `H_` | ㅎ | U+314E | U+1112 |

Jungseong (`O_ U_ A_ I_ E_ EU` + `YO YU YA YE YEO YAE AE EO EUI` + compounds in UnitMix):

| Mnem | Jamo | compat | conjoining V |
|---|---|---|---|
| `A_` | ㅏ | U+314F | U+1161 |
| `AE` | ㅐ | U+3150 | U+1162 |
| `YA` | ㅑ | U+3151 | U+1163 |
| `YAE` | ㅒ | U+3152 | U+1164 |
| `EO` | ㅓ | U+3153 | U+1165 |
| `E_` | ㅔ | U+3154 | U+1166 |
| `YEO` | ㅕ | U+3155 | U+1167 |
| `YE` | ㅖ | U+3156 | U+1168 |
| `O_` | ㅗ | U+3157 | U+1169 |
| `WA` | ㅘ | U+3158 | U+116A |
| `WAE` | ㅙ | U+3159 | U+116B |
| `OI` | ㅚ | U+315A | U+116C |
| `YO` | ㅛ | U+315B | U+116D |
| `U_` | ㅜ | U+315C | U+116E |
| `UEO`(워) | ㅝ | U+315D | U+116F |
| `WE` | ㅞ | U+315E | U+1170 |
| `WI` | ㅟ | U+315F | U+1171 |
| `YU` | ㅠ | U+3160 | U+1172 |
| `EU` | ㅡ | U+3161 | U+1173 |
| `EUI`(의) | ㅢ | U+3162 | U+1174 |
| `I_` | ㅣ | U+3163 | U+1175 |

Jongseong (used: `_G _GG _GS _N _NJ _NH _D _R _RG _RM _RB _RS _RT _RP _RH _M _B _BS _S _SS _J
_C _K _T _P _H`):

| Mnem | Jamo | compat | conjoining T |
|---|---|---|---|
| `_G` | ㄱ | U+3131 | U+11A8 |
| `_GG` | ㄲ | U+3132 | U+11A9 |
| `_GS` | ㄳ | U+3133 | U+11AA |
| `_N` | ㄴ | U+3134 | U+11AB |
| `_NJ` | ㄵ | U+3135 | U+11AC |
| `_NH` | ㄶ | U+3136 | U+11AD |
| `_D` | ㄷ | U+3137 | U+11AE |
| `_R` | ㄹ | U+3139 | U+11AF |
| `_RG` | ㄺ | U+313A | U+11B0 |
| `_RM` | ㄻ | U+313B | U+11B1 |
| `_RB` | ㄼ | U+313C | U+11B2 |
| `_RS` | ㄽ | U+313D | U+11B3 |
| `_RT` | ㄾ | U+313E | U+11B4 |
| `_RP` | ㄿ | U+313F | U+11B5 |
| `_RH` | ㅀ | U+3140 | U+11B6 |
| `_M` | ㅁ | U+3141 | U+11B7 |
| `_B` | ㅂ | U+3142 | U+11B8 |
| `_BS` | ㅄ | U+3144 | U+11B9 |
| `_S` | ㅅ | U+3145 | U+11BA |
| `_SS` | ㅆ | U+3146 | U+11BB |
| `_NG`/`_Q` | ㅇ | U+3147 | U+11BC |
| `_J` | ㅈ | U+3148 | U+11BD |
| `_C` | ㅊ | U+314A | U+11BE |
| `_K` | ㅋ | U+314B | U+11BF |
| `_T` | ㅌ | U+314C | U+11C0 |
| `_P` | ㅍ | U+314D | U+11C1 |
| `_H` | ㅎ | U+314E | U+11C2 |

> Confidence HIGH for the mapping itself (the mnemonics are transparently the McCune-style
> romanization the author uses elsewhere, and the key positions match the 세벌식 최종 layout:
> e.g. 0x6B 'k'→`G_`(ㄱ초성), 0x73 's'→`_N`(ㄴ받침), 0x66 'f'→`A_`(ㅏ), 0x74 't'→`EO`(ㅓ)).
> The exact spelling of a couple of rare jongseong mnemonics (`_RS _RT _RP`) is inferred by the
> obvious pattern; verify against the running editor if a round-trip matters.

### 2.2 Raw numeric jamo encodings `H3|0x……`

When no short mnemonic exists, the operand is a **raw 날개셋 jamo code** packed as
`(category << 16) | index` (high 16 bits select 초/중/종 + variant plane, low 16 bits select the
jamo). Observed:

| Value | Position in 세벌식-맞춤 | Hypothesis |
|---|---|---|
| `H3\|0x800000` | 0x76 'v' | a 초/특수 jamo (high byte 0x80 = base plane). 'v' in 세벌식 최종 = ㅎ초성-area / 옛 jamo |
| `H3\|0x810000` | 0x62 'b' | category-tagged jamo, plane 0x81 |
| `H3\|0x820000` | 0x67 'g' | category-tagged jamo, plane 0x82 |
| `H3\|0x1F4` | 0x24 `$` (mid-comp) | low-range code 0x1F4 = **500** → see §2.3 (this is the 같은-키/된소리 toggle operand fed as a jamo) |
| `H3\|_GG` etc. | — | mnemonic form of the same space |

`0x81xxxx`/`0x82xxxx`/`0x80xxxx` are the **category/plane bits**: high word ≠ 0 marks a jamo
that isn't one of the ~70 mnemonic units (옛한글 or position-variant). For a generic
interpreter, store the full 32-bit code and resolve it through the same unit tables; only the
low bits index the jamo and the high bits pick 초/중/종 (and old-hangul plane). Exact plane
semantics: **MEDIUM/LOW confidence** — decode empirically by feeding each into the composer.

> `0x800000`/`0x810000`/`0x820000` differ only in the high byte → they are three jamo in three
> different *category planes* (plausibly 초성/중성/종성 variants of a rare/old jamo, or the
> three 옛한글 representation planes the converter handles). They sit on keys b/g/v which in
> 세벌식 최종 carry 옛한글/extension jamo. Treat the high byte as the category selector.

---

## 3. AutomataTable — the composition state machine

```xml
<AutomataTable default="0">
  <Automata state="0" value="1" default="0" remark="초기 상태"/>
  <Automata state="1"
    value="D==176&&A==176 || D==185&&A==185 ? 0 : A||B||C ? (A||D)&&(B||E) ? 2 : 1 : -2"
    default="-1" remark="미완성 상태"/>
  <Automata state="2"
    value="A&&A!=500 ? 0 : B||C||A==500 ? 2 : -2"
    default="0" remark="한글 완성 상태"/>
</AutomataTable>
```

### 3.1 Model

Each `<Automata>` row is the transition rule **for one current state**. When a `H3|`unit key
arrives while the composer is in state *S*, the engine evaluates row *S*'s `value` expression;
the integer result is the **next action / state**. If the expression cannot apply (or the input
isn't a Hangul unit), `default` is used. `AutomataTable default="0"` = initial machine state.

`state` ids here: **0 = 초기(empty), 1 = 미완성(incomplete: has some jamo but not a full
syllable), 2 = 한글 완성(complete syllable formed)**. These are exactly the values the key
expressions read as `T` (T = current state id; T≠0 ⇒ composing).

### 3.2 The variables A,B,C,D,E

Derived from the formulas + 한글 오토마타 theory + the official A/B/E hints. Each is a
**predicate about a unit code** (so it's also usable in equality tests like `A==176`):

- **`A`** = the **incoming unit, viewed as a choseong** — non-zero (truthy) iff the just-pressed
  unit *can be a 초성*; its value is the unit's choseong code (hence `A==176`, `A==500`,
  `A!=500` comparisons).
- **`B`** = the incoming unit viewed as a **중성** — truthy iff it can be a jungseong.
- **`C`** = the incoming unit viewed as a **종성** — truthy iff it can be a jongseong.
- **`D`** = the **current syllable already has a 초성** (non-zero = its choseong code).
- **`E`** = the current syllable already has a **중성** (non-zero = its jungseong code). This is
  the same `E` used officially in `T && E==0` ("중성이 전혀 입력되지 않았을 때").

So A/B/C describe the *new* unit's possible roles, D/E describe what slots the *current*
syllable already holds. (A "unit" in 세벌식 may be ambiguous — a key can be only-choseong,
only-jungseong, etc.; A/B/C are the per-category projections.)

> Confidence: A,B,C = incoming-can-be-cho/jung/jong and D,E = current-has-cho/has-jung is a
> data-derived hypothesis, strongly consistent with every clause below and with the official E
> meaning. MEDIUM-HIGH. The precise definition of whether A/B/C carry the *code* vs a bare bool
> is inferred from the `A==176`/`A==500` comparisons (they carry the code). Verify against the
> editor's automata help if exactness is needed.

### 3.3 Result (next-state) codes

- `0` = **commit current syllable and start fresh, feeding this unit into a new syllable**
  (i.e. "flush + this unit begins state 1/2"). Used when the incoming unit must begin a *new*
  character.
- `1` = stay/become **미완성** (incomplete) — keep composing, syllable not yet complete.
- `2` = become/stay **완성** (complete syllable).
- `-1` = the row `default` for state 1: **commit/flush** (no Hangul action; pass through). In
  날개셋 negative results = "조합 종료/거부" style outcomes.
- `-2` = **reject / non-Hangul** → the unit is not composable here; commit what exists and emit
  the key literally (treated as boundary/flush of the composition with the input not consumed as
  jamo). Both `-1` and `-2` are "end composition" signals; the magnitude distinguishes flavours
  (e.g. whether the triggering key is re-processed). 

> Confidence: 0/1/2 = state ids is certain (they equal the row ids & remarks). Negative = end/
> reject is HIGH (matches author's "조합 종료" semantics and the only sensible reading); the
> precise -1 vs -2 distinction (commit-and-reprocess vs commit-and-pass) is MEDIUM.

### 3.4 Worked reading of each row

**State 0 (초기):** `value="1"` default `0`. Any Hangul unit → go to state **1** (begin a
syllable). Non-unit → stay 0.

**State 1 (미완성):**
```
D==176 && A==176  ||  D==185 && A==185 ? 0
: A||B||C ? (A||D)&&(B||E) ? 2 : 1
: -2
```
- `176 = 0xB0`, `185 = 0xB9`. These are specific **jungseong unit codes**: in the 날개셋
  jungseong numbering these are the two compound-vowel "anchor" vowels **ㅗ (O_) and ㅜ (U_)**
  families — the vowels that form 이중모음 (ㅘ/ㅙ/ㅚ from ㅗ, ㅝ/ㅞ/ㅟ from ㅜ). The clause
  `D==176&&A==176 || D==185&&A==185` reads: *if the current 중성 anchor is ㅗ and another ㅗ
  arrives* (or ㅜ+ㅜ) → result `0` (don't double the same compound base; commit & restart).
  This is the guard that prevents ㅗ+ㅗ / ㅜ+ㅜ from illegally combining. (D here is reused as
  "current 중성 code" in the vowel context — i.e. D/E are the current cho/jung slots and the
  equality test inspects the relevant one.)

  > Caveat: whether 176/185 are jungseong codes for ㅗ/ㅜ specifically is a **hypothesis**
  > (MEDIUM): they're clearly two specific unit codes that pair only with themselves, and the
  > only Korean composition rule of that shape is the ㅗ/ㅜ self-collision guard for compound
  > vowels. The literal numbers 0xB0/0xB9 should be confirmed against the unit-code table.
- Else `A||B||C` (incoming is a real Hangul unit): if `(A||D) && (B||E)` — i.e. *(there is now
  a choseong: either incoming-as-cho or already-present)* **and** *(there is now a jungseong)* —
  the syllable has both 초성+중성 ⇒ **complete → 2**; otherwise still incomplete ⇒ **1**.
- Else (`A||B||C` false → not a Hangul unit): **-2** (reject/flush). Row `default="-1"` covers
  the case where the rule itself doesn't fire (non-unit event).

**State 2 (완성):**
```
A && A!=500 ? 0 : B||C||A==500 ? 2 : -2
```
- `A && A!=500`: incoming is a **plain choseong (and not the special 500 toggle)** → a new
  consonant after a complete syllable starts the **next** character ⇒ commit & restart **0**.
- else `B||C||A==500`: incoming is a **중성, a 종성, or the 500 toggle** → it can still attach to
  the *current* complete syllable (a 받침 on a CV syllable, or a 된소리 toggle) ⇒ stay **2**.
- else **-2** (reject). default `0`.

This precisely encodes the 세벌식 behaviour ngs_menu2 describes ("초성이 입력되었을 때만 다음
글자로 넘어가고 중성·종성은 현재 글자를 계속 고친다").

---

## 4. FinalConvTable — conjoining/old jamo → compatibility jamo

```xml
<FinalConv from="0x1100" to="0x3131"/>   …   <FinalConv from="0xECBF" to="0x3185"/>
```

**Direction:** `from` = a **conjoining/positional or old-hangul jamo code**, `to` = the
**compatibility jamo** (U+31xx, the standalone "기본" form). **Applied when a *standalone /
incomplete* jamo must be output** — i.e. when the composer has a lone 초성/중성/종성 (or an old-
hangul leftover) that isn't part of a finished syllable, it is normalised to its
compatibility-jamo glyph so it renders as a normal ㄱ/ㅏ/… instead of an invisible conjoining
form. This is the 채움문자/미완성→완성자 normalisation referenced officially ("한글 채움 문자가
포함된 글자를 완성자로 변환", ngs_imple).

`from` value ranges and what they cover (all → U+31xx compat jamo):
- **U+1100–U+1112** = conjoining **choseong** (L) → compat consonants.
- **U+1113–U+115C** = old/extended **choseong** (옛한글) → compat (e.g. 0x111A ㅀ-cluster,
  0x1140 ㅿ→U+317F, 0x114C ㆁ→U+3181).
- **U+1161–U+1175** = conjoining **jungseong** (V) → compat vowels (U+314F–U+3163).
- **U+1184–U+11A1** = old **jungseong** → compat (e.g. 0x119E ㆍ아래아 → U+318D).
- **U+11A8–U+11C2** = conjoining **jongseong** (T) → compat (note jongseong ㄱ U+11A8 → U+3131,
  the *same* compat glyph as choseong ㄱ — compatibility jamo don't distinguish position).
- **U+11C3–U+11FF** = old **jongseong** → compat.
- **0xA9xx (A964…A971)** = Hangul Jamo Extended-A (옛한글 받침) → compat.
- **0xD7xx (D7CD…D7F9)** = Hangul Jamo Extended-B (옛한글 초/중/종성) → compat.
- **0xEAxx / 0xECxx (PUA)** = 날개셋's private-use 옛한글 codepoints (the non-standard 한양 PUA-
  style plane the converter supports) → compat.

So the table is a **flatten-to-compat-jamo lookup** keyed by every jamo codepoint the engine can
hold internally. It is *not* a composition table; it only affects how an *uncombined* jamo is
rendered/emitted. Confidence HIGH (the from/to ranges are unambiguous Unicode block mappings).

For a Rust interpreter: load as a `HashMap<u32,u32>`; apply when emitting a syllable buffer that
contains a single unconsumed jamo (or when 채움-conversion is requested).

---

## 5. Extra / Bksp — backspace deletion granularity

```xml
<Extra>
  <Bksp key="1" value1="BkspAttach"            value2="ByUnitStep|BkspAttach" condition1="ReverseJLTRN" condition2="0"/>
  <Bksp key="2" value1="BySyllable|BkspAttach" value2="ByUnitStep|BkspAttach" condition1="ReverseJLTRN" condition2="0"/>
  <Bksp key="3" value1="0"                     value2="BySyllable"            condition1="0"            condition2="0"/>
  <Bksp key="4" value1="0"                     value2="BySyllable"            condition1="0"            condition2="0"/>
</Extra>
```

There are **4 backspace "slots"** (`key="1".."4"`). The 4 correspond to the four backspace-
behaviour situations the 'Bksp 동작 방식' dialog configures (ngs_menu1 'ngs_bkspdlg'); commonly:
1 = Bksp while composing, 2 = Bksp on a just-finished syllable, 3 = Bksp at a syllable boundary /
inside committed text, 4 = a secondary/forward case. (Exact slot→situation mapping: MEDIUM.)

Each slot has a **primary** action (`value1`, used when `condition1` holds) and a **fallback**
(`value2`, used when `condition2` holds / else). Values are **OR-able flag sets**:

| Flag | Meaning |
|---|---|
| `0` | default / no special handling (delete one char) |
| `BkspAttach` | after deleting, **re-enter composition** of the preceding syllable (the headline "Backspace로 완성된 글자도 낱자 단위로 재조합" feature, ngs_menu2 #1) — "attach" cursor back into the syllable |
| `ByUnitStep` | delete **one 낱자 (jamo) at a time**, replaying composition order (낱자 단위 삭제) |
| `BySyllable` | delete a **whole 음절(syllable)** at once |

Conditions:
| Condition | Meaning |
|---|---|
| `0` | always (unconditional) |
| `ReverseJLTRN` | apply only when **세벌식 역순 조합(reverse J/L… combination)** is in effect — i.e. the "세벌식 역순 조합 허용" 낱자-처리 option (confirmed term in community setup guides). Roughly: the syllable was built in reverse (종성→중성→초성) order, so the unit-step deletion must walk it in the matching reverse order. |

So entry #0 (세벌식): slots 1&2 do **낱자-단위 재조합 삭제 with re-attach** when reverse-combine
is active (else fall back to unit-step+attach); slots 3&4 delete by whole syllable. Entry #1
(Latin) uses `0 / BySyllable` everywhere (plain deletion). Confidence: flag *names* are self-
descriptive (HIGH); the precise slot semantics and `ReverseJLTRN` trigger condition are
MEDIUM — verify against the bksp dialog.

---

## 6. ShortcutTable (Editor Layer global keys)

```xml
<Shortcut key="VK_HANGUL"  usage="IME_SWITCH" value="!A"/>
<Shortcut key="VK_HANJA"   usage="KEYCHAR"    value="C0|0x82"/>
<Shortcut key="VK_CAPITAL" modifier="DONT_EAT|KEEP_LAMP" usage="IME_SWITCH" value="!A"/>
```

Global keys recognised before any input entry (the Editor-Layer 단축글쇠, ngs_menu1
'ngs_shrtcut': "입력 항목의 전환과 한자 변환(C0|0x8?)이 대표적").

- **`key`** = a Windows virtual-key (`VK_HANGUL`=한/영, `VK_HANJA`=한자, `VK_CAPITAL`=CapsLock).
- **`usage`** selects how `value` is interpreted:
  - `IME_SWITCH` → `value` is an **input-entry switch command**. `!A` = "switch input entry" —
    the `!` form is the toggle/cycle among registered entries (the author calls it switching
    between 0↔1 or 2↔3, ngs_menu1). Here both 한/영 and CapsLock toggle the active entry. (`!A`
    vs `!B`… selects which switch group; `A` = first/default group.)
  - `KEYCHAR` → `value` is a **character/command value** injected as if typed: `C0|0x82` =
    **한자 변환** command. So pressing the 한자 key fires the Hanja-conversion special command.
- **`modifier="DONT_EAT|KEEP_LAMP"`** (on CapsLock): `DONT_EAT` = **don't swallow** the key —
  let it also pass through to the OS (so CapsLock still toggles the lock); `KEEP_LAMP` = **keep
  the Caps lock lamp/LED state** (don't disturb the indicator) while still using the key as an
  IME switch. (Names self-descriptive; HIGH.)

`!A` value form: `!` + letter = "activate input entry / switch set". This is the same family as
the `InputLayer current` index; an interpreter should map `!A`→switch-to-entry action.

---

## 7. UnitMixTable + VirtualUnitTable (낱자 결합)

### 7.1 UnitMix

`<UnitMix unit="CHO|JUNG|JONG" a="X" b="Y" to="Z"/>` = "within category `unit`, combining unit
`a` with subsequently-pressed unit `b` yields unit `Z`." (낱자 결합 규칙 = "한 타로 입력할 수
없는 복잡한 낱자를 결합으로 입력", ngs_menu1.)

Normal combinations in this file:
- **CHO** doubles via same-key: `G_+G_→GG`, `D_+D_→DD`, `B_+B_→BB`, `S_+S_→SS`, `J_+J_→JJ`
  (연타 된소리).
- **JUNG** compounds: `O_+A_→WA(ㅘ)`, `O_+AE→WAE(ㅙ)`, `O_+I_→OI(ㅚ)`, `U_+EO→UEO(ㅝ)`,
  `U_+E_→WE(ㅞ)`, `U_+I_→WI(ㅟ)`.
- **JONG** clusters: `R_+S_→RS(ㄽ)`, `R_+T_→RT(ㄾ)`, `R_+P_→RP(ㄿ)`. (Note jongseong combine
  uses the *consonant* mnemonics `R_`,`S_`,… as operands even though result is a 받침 — the
  category context = JONG.)

### 7.2 The special operand `b="500"` — 같은-키(갈마들이) 된소리 toggle

```xml
<UnitMix unit="CHO" a="G_"  b="500" to="GG"/>
<UnitMix unit="CHO" a="GG"  b="500" to="G_"/>
```
…and similarly for D/B/S/J. **`500` is a sentinel "same-key / toggle" pseudo-unit**, not a real
jamo. It means "the *same key was pressed again*." So:
- `G_ + (same key) → GG` (plain → 된소리), and
- `GG + (same key) → G_` (된소리 → plain).

Together this is the **갈마들이/같은-글쇠 된소리 토글**: tapping the ㄱ key cycles ㄱ→ㄲ→ㄱ→…
(used by 세벌식 변형들 and 신세벌식 where one key serves both, instead of Shift). The `b="G_"`
rows give the *연타* path (ㄱ then ㄱ) while the `b="500"` rows give the *same-physical-key
re-press* path; both produce ㄲ. 500 is fed as `H3|0x1F4` from a key (0x1F4 = 500) — that's why
key 0x24 has `T ? H3|0x1F4 : '$'`: mid-composition that key emits the 500 toggle unit.

This also explains the automata State 2 clause `A==500 ? 2`: a 500 (same-key toggle) arriving on
a complete syllable stays in state 2 (it mutates the existing 초성's tension, doesn't start a new
char), whereas a *real* choseong (`A && A!=500`) → 0 (new char).

> Confidence HIGH: the `(X,500→XX)` / `(XX,500→X)` symmetric pairs are unambiguously a
> same-key toggle; 500=0x1F4 ties directly to `H3|0x1F4`. The label "갈마들이/같은 글쇠 된소리"
> is the standard 세벌식 term for it.

### 7.3 VirtualUnitTable

```xml
<VirtualUnit unit="JUNG" from="128" to="O_"/>
<VirtualUnit unit="JUNG" from="129" to="U_"/>
<VirtualUnit unit="JUNG" from="130" to="EU"/>
```
A **virtual unit** = an internal id (`from`, here 128/129/130) that is **aliased to a real unit**
(`to`) within a category. These let the automata / key tables refer to a vowel by a stable
internal slot regardless of which concrete jamo currently fills it — specifically the three
**compound-vowel anchor vowels ㅗ/ㅜ/ㅡ** (the vowels that take a following vowel to form
ㅘ/ㅙ/ㅚ, ㅝ/ㅞ/ㅟ, ㅢ). 128→ㅗ(O_), 129→ㅜ(U_), 130→ㅡ(EU). They are the "first half of a
compound vowel" placeholders the 모아치기/compound logic uses. (This dovetails with §3.4's
176/185 ㅗ/ㅜ self-collision guard: virtual ids 128/129 mark exactly those anchors.)

> Confidence: virtual=alias is HIGH; that 128/129/130 = the ㅗ/ㅜ/ㅡ compound anchors is given
> directly by the `to=` values. Their relationship to the automata's 176/185 constants is a
> reasoned link (MEDIUM) — i.e. 176/185 are likely the *resolved* jungseong codes for these
> anchors, but the exact numbering should be confirmed.

---

## 8. Interpreter implementation checklist (Rust)

1. **Expression evaluator**: a small C-operator Pratt parser over tokens `{number(dec/0xhex),
   ident(T,P,A..E,…), H3|operand, C0|n, !A, ( ) ? : || && | ^ & == != < > <= >= << >> + - * / %
   ! ~}`. Result = `i64`/tagged value. Context supplies T,P,A,B,C,D,E.
2. **Value tag**: result carries a kind — `Char(u32)` (literal/computed), `Unit(u32)` (from
   `H3|`), `Command(u16)` (from `C0|`), `SwitchEntry(idx)` (from `!A`).
3. **Unit codes**: build mnemonic→code tables (§2.1) and handle raw `0x……` codes (§2.2). Keep
   the 500 sentinel and virtual-unit aliases (§7.3) in the same space.
4. **Automata**: per current-state row, evaluate `value` with A/B/C = incoming-unit-as-
   cho/jung/jong and D/E = current cho/jung slots; interpret result {0=flush+restart,
   1=incomplete, 2=complete, <0=end/reject}; fall back to `default` when the unit isn't
   composable. Set `T` = current state id for key-expr evaluation.
5. **UnitMix**: on a second unit in the same category, look up `(a,b)`; treat `b=500` as the
   same-key re-press; produce `to`.
6. **FinalConv**: `HashMap<u32,u32>`; apply when emitting an uncombined/standalone jamo.
7. **Bksp**: 4 slots, each `(value1@condition1 else value2@condition2)` of OR-flags
   {BkspAttach, ByUnitStep, BySyllable}; `ReverseJLTRN` gates on the reverse-combine option.
8. **Shortcuts**: VK→(usage,value,modifier); `IME_SWITCH`+`!A`=cycle entries,
   `KEYCHAR`+`C0|n`=inject command; honour `DONT_EAT`/`KEEP_LAMP`.

---

## 9. Confidence summary

| Claim | Confidence | Basis |
|---|---|---|
| Expr language = C operators; result int | HIGH | official ngs_menu1 |
| T = automaton state (T≠0 = composing) | HIGH | official |
| E = 중성 present | HIGH | official example |
| P = modifier bitmask, P&1=Shift, ^(P&1)<<5 = case toggle | HIGH | data + official Caps/Num note |
| H3\| = jamo unit, C0\| = command, C0\|0x82=Hanja | HIGH | official `H3\|O`, `C0\|0x8?` |
| Mnemonic→jamo table | HIGH | transparent romanization + key-position cross-check |
| 500 = same-key 된소리 toggle sentinel | HIGH | symmetric UnitMix pairs + `H3\|0x1F4` |
| VirtualUnit = alias; 128/129/130 = ㅗ/ㅜ/ㅡ anchors | HIGH | direct `to=` values |
| A/B/C=incoming-as-cho/jung/jong, D/E=current cho/jung | MEDIUM-HIGH | data-derived, fits all clauses + official E |
| Automata result codes 0/1/2/-1/-2 meanings | HIGH for 0/1/2, MEDIUM for −1 vs −2 | row ids/remarks; sign convention |
| 176/185 = ㅗ/ㅜ jungseong codes (compound guard) | MEDIUM | only rule of that shape; raw numbers unverified |
| FinalConv direction/blocks | HIGH | unambiguous Unicode ranges |
| Bksp flag meanings & ReverseJLTRN | MEDIUM | self-descriptive names + community term |
| Exact C0\| command ids (other than 0x82) | LOW | undocumented; verify empirically |
| 0x80/0x81/0x82xxxx plane semantics | LOW-MEDIUM | high-byte = category/plane inference |
