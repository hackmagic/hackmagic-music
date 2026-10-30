#!/usr/bin/env bash
# =============================================================================
# HackMagic Music Player — 验收闸门（自动部分）
#
# 配套文档: docs/验收标准.md
# 用法:
#   ./verify.sh            # 快速闸门: 只跑静态检查（秒级，不编译）
#   ./verify.sh --full     # 完整闸门: 静态检查 + clippy + 单元测试
#   ./verify.sh --size     # 额外校验 release 产物体积（需已 cargo build --release）
#
# 退出码: 0 = 全部通过; 1 = 存在 FAIL（改动不得提交、不得计入"已完成"）
# =============================================================================

# 本机 bash 缺 coreutils（dirname/ls/grep 等全在 Git 的 usr/bin 下），
# 不补 PATH 脚本会立刻失败。
# 注意：不要硬编码 PortableGit 的版本号目录（曾写死 1.2.0，该目录被删除后
# 闸门莫名全红，与代码质量无关）。改为「探测 + 回退」。
COREUTILS=""
for _cand in \
  "/c/Users/angel/.workbuddy/binaries/PortableGit/versions/current/usr/bin" \
  "/d/SystemSoftware/Git/Git/usr/bin" \
  "/c/Program Files/Git/usr/bin" ; do
  if [ -x "$_cand/grep.exe" ]; then COREUTILS="$_cand"; break; fi
done
# 上述候选全 miss 时，扫描 PortableGit 的版本目录（版本号会随升级变化）
if [ -z "$COREUTILS" ]; then
  for _v in /c/Users/angel/.workbuddy/binaries/PortableGit/versions/*/usr/bin; do
    if [ -x "$_v/grep.exe" ]; then COREUTILS="$_v"; break; fi
  done
fi

# ---- cargo 定位（关键：绝不能静默缺失） ----
# 教训：通过 PowerShell 调本脚本时，系统 PATH 里的 bash.exe 可能解析到 WSL
# （$HOME=/root，$HOME/.cargo/bin 不存在）。此时 cargo not found 会让
# clippy 输出为空 → nw=0 → "0 警告 0 错误" 假阳性 PASS。因此 cargo 必须
# 显式探测，且探测失败时 G0-2/G0-3 直接 FAIL，不允许退化成空输出。
CARGO_BIN=""
if command -v cargo >/dev/null 2>&1; then
  CARGO_BIN="cargo"
else
  # msys bash 用 /c 挂载，WSL bash 用 /mnt/c，两种都要试
  for _cd in \
    "/c/Users/angel/.cargo/bin" \
    "/mnt/c/Users/angel/.cargo/bin" \
    "/d/SystemSoftware/.cargo/bin" \
    "/mnt/d/SystemSoftware/.cargo/bin" \
    "$USERPROFILE/.cargo/bin" ; do
    if [ -x "$_cd/cargo.exe" ]; then CARGO_BIN="$_cd/cargo.exe"; break; fi
  done
fi

if [ -n "$COREUTILS" ]; then
  # 只补 usr/bin：mingw64 里的同名工具会与 usr/bin 版本打架
  export PATH="$COREUTILS:$PATH"
fi
# 探测失败则依赖 bash 自带的 coreutils（msys bash 通常自带）；若仍缺工具，
# 后续 grep/find 会以可读错误退出，不会静默通过。
# G0-7 用 UTF-8 字符范围 [一-龥] 检测硬编码中文；locale 回落到 C/POSIX 时
# grep 按字节匹配，会把 ⏮ ▶ ℹ 等符号误判为命中。固定 UTF-8 locale 防止误报。
export LC_ALL=C.UTF-8
export LANG=C.UTF-8

cd "$(dirname "$0")" || exit 1

# ---- release 体积基线（字节），变更时同步 docs/验收标准.md §5 ----
BASELINE_GUI=15575552   # hm-gui.exe
BASELINE_CLI=8126464    # hm.exe
SIZE_TOLERANCE=5        # 允许 +5% 回归
UNWRAP_BASELINE=369     # 生产代码 unwrap/expect 数量棘轮（不得增加），见 docs/验收标准.md §2
TEST_BASELINE=593       # 单元测试通过数下限（只能增加）

FULL=0; SIZE=0
for a in "$@"; do
  case "$a" in
    --full) FULL=1 ;;
    --size) SIZE=1 ;;
    *) echo "未知参数: $a"; exit 2 ;;
  esac
done

PASS=0; FAIL=0; SKIP=0
ok()   { printf '  \033[32mPASS\033[0m  %s\n' "$1"; PASS=$((PASS+1)); }
bad()  { printf '  \033[31mFAIL\033[0m  %s\n' "$1"; FAIL=$((FAIL+1)); }
skip() { printf '  \033[33mSKIP\033[0m  %s\n' "$1"; SKIP=$((SKIP+1)); }
hdr()  { printf '\n\033[1m%s\033[0m\n' "$1"; }

# 统计生产代码中的 unwrap()/expect()，跳过 #[cfg(test)] 块
count_prod_unwrap() {
  find src -name '*.rs' -print0 2>/dev/null | xargs -0 awk '
    FNR==1 { intest=0; depth=0; started=0 }
    BEGIN { n=0 }
    {
      line=$0
      if (intest==0 && line ~ /^[[:space:]]*#\[cfg\(test\)\]/) { intest=1; depth=0; started=0; next }
      if (intest==1) {
        o=gsub(/\{/,"{"); c=gsub(/\}/,"}")
        depth+=o-c
        if (o>0) started=1
        if (started && depth<=0) { intest=0; started=0 }
        next
      }
      if (line ~ /\.unwrap\(\)|\.expect\(/) n++
    }
    END { print n+0 }
  ' | awk '{ s+=$1 } END { print s+0 }'
}

# =============================================================================
hdr "G0 静态闸门"
# =============================================================================

# G0-5 catch_unwind 必须缺席 —— release 档开了 panic="abort"，一旦引入 catch_unwind
# 会静默失效（catch 不到 panic，直接 abort）。
cc=$(grep -rl "catch_unwind" src --include='*.rs' 2>/dev/null | wc -l | tr -d ' ')
if [ "$cc" = "0" ]; then
  ok "G0-5 无 catch_unwind（panic=abort 前提成立）"
else
  bad "G0-5 发现 catch_unwind（$cc 个文件）→ panic=abort 会让它静默失效"
  grep -rn "catch_unwind" src --include='*.rs' | head -5 | sed 's/^/        /'
fi

# G0-6 GUI 不得用 println!/eprintln!（必须走 tracing）
pn=$(grep -rn "println!\|eprintln!" src/gui --include='*.rs' 2>/dev/null | wc -l | tr -d ' ')
if [ "$pn" = "0" ]; then
  ok "G0-6 src/gui 无 println!/eprintln!（走 tracing）"
else
  bad "G0-6 src/gui 出现 $pn 处 println!/eprintln!"
  grep -rn "println!\|eprintln!" src/gui --include='*.rs' | head -5 | sed 's/^/        /'
fi

# G0-7 GUI 面向用户的文案不得硬编码中文（必须走 gui/i18n.rs 的 Tr）
cjk=$(grep -rnE '\.(label|placeholder|title)\(\s*"[^"]*[一-龥]|layout::txt\(\s*"[^"]*[一-龥]|PopupMenuItem::new\(\s*"[^"]*[一-龥]' \
        src/gui --include='*.rs' 2>/dev/null | grep -v 'i18n.rs' | wc -l | tr -d ' ')
if [ "$cjk" = "0" ]; then
  ok "G0-7 GUI 文案无硬编码中文（i18n 双份由编译器强制）"
else
  bad "G0-7 GUI 出现 $cjk 处硬编码中文文案"
  grep -rnE '\.(label|placeholder|title)\(\s*"[^"]*[一-龥]|layout::txt\(\s*"[^"]*[一-龥]|PopupMenuItem::new\(\s*"[^"]*[一-龥]' \
    src/gui --include='*.rs' | grep -v 'i18n.rs' | head -5 | sed 's/^/        /'
fi

# G0-8 unwrap/expect 棘轮：只允许减少
uw=$(count_prod_unwrap)
if [ "$uw" -le "$UNWRAP_BASELINE" ]; then
  ok "G0-8 生产代码 unwrap/expect = $uw（基线 $UNWRAP_BASELINE，只允许下降）"
else
  bad "G0-8 生产代码 unwrap/expect = $uw > 基线 $UNWRAP_BASELINE（新增了违规）"
fi

# =============================================================================
hdr "G0 编译闸门"
# =============================================================================

if [ "$FULL" = "1" ]; then
  # G0-2 clippy: 必须 0 警告 0 错误
  if [ -z "$CARGO_BIN" ]; then
    bad "G0-2 找不到 cargo.exe（PATH 与 .cargo/bin 均无）→ 拒绝假阳性 PASS"
  else
  clip=$("$CARGO_BIN" clippy --all-targets --message-format short 2>&1)
  # 只统计「指向本仓库源码」的诊断（src/ 与 tests/），外加硬编译错误。
  #
  # 教训（§14.5 的反面）：旧写法 `grep -cE '^(warning|error)'` 会把
  #   warning: the following packages contain code that will be rejected by a future
  #   version of Rust: proc-macro-error2 v2.0.1
  # 这条**依赖包**的 future-incompat 提示也算成「我们的警告」→ 闸门误判失败。
  # 该依赖来自 gpui 0.2.2 → stacksafe → proc-macro-error2，gpui 版本被钉死、
  # proc-macro-error2 2.0.1 已是 registry 上的最新版 → 本仓无法消除。
  # 闸门要管的是「我们自己的代码 0 警告」，依赖噪音不算（见 docs/验收标准.md §6）。
  nw=$(printf '%s\n' "$clip" | grep -cE '^(src|tests)[\\/][^ ]*: (warning|error)|^error(\[|:)' || true)
  if [ "$nw" = "0" ]; then
    ok "G0-2 cargo clippy --all-targets 0 警告 0 错误"
  else
    bad "G0-2 clippy 有 $nw 条警告/错误"
    printf '%s\n' "$clip" | grep -E '^(src|tests)[\\/][^ ]*: (warning|error)|^error(\[|:)' | head -8 | sed 's/^/        /'
  fi
  fi

  # G0-3 单元测试: 全绿且通过数不低于基线
  tst=$("$CARGO_BIN" test --lib 2>&1)
  line=$(printf '%s\n' "$tst" | grep -E '^test result:' | head -1)
  np=$(printf '%s\n' "$line" | sed -n 's/.*ok\. \([0-9]*\) passed.*/\1/p')
  nf=$(printf '%s\n' "$line" | sed -n 's/.* \([0-9]*\) failed.*/\1/p')
  np=${np:-0}; nf=${nf:-1}
  if [ "$nf" = "0" ] && [ "$np" -ge "$TEST_BASELINE" ]; then
    ok "G0-3 cargo test --lib: $np 通过 / $nf 失败（下限 $TEST_BASELINE）"
  else
    bad "G0-3 cargo test --lib: $np 通过 / $nf 失败（下限 $TEST_BASELINE）"
    printf '%s\n' "$tst" | grep -E '^test .* FAILED|^failures:' | head -8 | sed 's/^/        /'
  fi
else
  skip "G0-2/G0-3 clippy 与单元测试（加 --full 启用）"
fi

# =============================================================================
hdr "G3 交付闸门（release 体积）"
# =============================================================================
if [ "$SIZE" = "1" ]; then
  check_size() { # $1=文件 $2=基线 $3=名称
    if [ ! -f "$1" ]; then skip "$3 未构建（先 cargo build --release）"; return; fi
    cur=$(stat -c %s "$1" 2>/dev/null || echo 0)
    lim=$(( $2 * (100 + SIZE_TOLERANCE) / 100 ))
    if [ "$cur" -le "$lim" ]; then
      ok "$3 = $cur 字节（基线 $2，上限 $lim）"
    else
      bad "$3 = $cur 字节 > 上限 $lim（体积回归）"
    fi
  }
  check_size target/release/hm-gui.exe "$BASELINE_GUI" "hm-gui.exe"
  check_size target/release/hm.exe     "$BASELINE_CLI" "hm.exe"
else
  skip "release 体积校验（加 --size 启用）"
fi

# =============================================================================
hdr "汇总"
printf '  通过 %d   失败 %d   跳过 %d\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" -gt 0 ]; then
  printf '\n\033[31m闸门未通过 —— 不得提交、不得计入"已完成"。\033[0m\n'
  printf '功能级验收（docs/验收标准.md §4）仍需人工执行并写入 §7 验收记录。\n'
  exit 1
fi
printf '\n\033[32m自动闸门全部通过。\033[0m\n'
printf '下一步：按 docs/验收标准.md §4 执行本次改动涉及的模块验收，并追加 §7 验收记录。\n'
exit 0
