#!/usr/bin/env bash
# install/install.sh — install external dependencies and build scientist-in-loop (`sil`)
#
# Supported hosts:
#   macOS   (Homebrew)
#   Linux   (apt, dnf/yum, pacman, zypper, apk)
#   Windows (Git Bash / MSYS2 / Cygwin; WSL uses the Linux path)
#
# Usage:
#   ./install/install.sh                 # core deps + build sil
#   ./install/install.sh --with-marker   # also install marker-pdf (heavy)
#   ./install/install.sh --with-latex    # also install tectonic (or TeX fallback)
#   ./install/install.sh --check-only    # report what is present/missing
#   ./install/install.sh --skip-build    # deps only, do not cargo-install sil
#   ./install/install.sh --help
#
# Environment:
#   SIL_PYTHON          Python executable (default: python3, then python)
#   SKIP_MARKER=1       Same as omitting --with-marker
#   SKIP_LATEX=1        Same as omitting --with-latex
#   NONINTERACTIVE=1    Never prompt; assume yes for package installs when possible

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

WITH_MARKER=0
WITH_LATEX=0
CHECK_ONLY=0
SKIP_BUILD=0
NONINTERACTIVE="${NONINTERACTIVE:-0}"

usage() {
  cat <<'EOF'
install/install.sh — install scientist-in-loop external dependencies and build `sil`

Options:
  --with-marker    Install marker-pdf via pip (preferred PDF parser; large download)
  --with-latex     Install a LaTeX engine (tectonic preferred) for `sil build`
  --check-only     Print dependency status and exit (no installs)
  --skip-build     Install/check deps only; do not compile/install the sil binary
  --help           Show this help

Examples:
  ./install/install.sh
  ./install/install.sh --with-marker --with-latex
  ./install/install.sh --check-only
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --with-marker) WITH_MARKER=1; shift ;;
    --with-latex)  WITH_LATEX=1; shift ;;
    --check-only)  CHECK_ONLY=1; shift ;;
    --skip-build)  SKIP_BUILD=1; shift ;;
    -h|--help)     usage; exit 0 ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "${SKIP_MARKER:-0}" == "1" ]]; then WITH_MARKER=0; fi
if [[ "${SKIP_LATEX:-0}" == "1" ]]; then WITH_LATEX=0; fi

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
if [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]]; then
  C_RESET=$'\033[0m'
  C_BOLD=$'\033[1m'
  C_GREEN=$'\033[32m'
  C_YELLOW=$'\033[33m'
  C_RED=$'\033[31m'
  C_CYAN=$'\033[36m'
  C_DIM=$'\033[2m'
else
  C_RESET= C_BOLD= C_GREEN= C_YELLOW= C_RED= C_CYAN= C_DIM=
fi

info()  { printf '%s==>%s %s\n' "${C_BOLD}${C_CYAN}" "${C_RESET}" "$*"; }
ok()    { printf '%s[ok]%s %s\n' "${C_GREEN}" "${C_RESET}" "$*"; }
warn()  { printf '%s[warn]%s %s\n' "${C_YELLOW}" "${C_RESET}" "$*"; }
err()   { printf '%s[error]%s %s\n' "${C_RED}" "${C_RESET}" "$*" >&2; }
dim()   { printf '%s    %s%s\n' "${C_DIM}" "$*" "${C_RESET}"; }

have() { command -v "$1" >/dev/null 2>&1; }

# ---------------------------------------------------------------------------
# OS detection
# ---------------------------------------------------------------------------
OS_FAMILY=unknown   # macos | linux | windows
PKG=none            # brew | apt | dnf | yum | pacman | zypper | apk | choco | scoop | winget | msys2
UNAME_S="$(uname -s 2>/dev/null || echo unknown)"

detect_os() {
  case "${UNAME_S}" in
    Darwin)
      OS_FAMILY=macos
      if have brew; then PKG=brew; fi
      ;;
    Linux)
      OS_FAMILY=linux
      # WSL reports Linux; treat as Linux for package managers
      if have apt-get; then PKG=apt
      elif have dnf; then PKG=dnf
      elif have yum; then PKG=yum
      elif have pacman; then PKG=pacman
      elif have zypper; then PKG=zypper
      elif have apk; then PKG=apk
      fi
      ;;
    MINGW*|MSYS*|CYGWIN*)
      OS_FAMILY=windows
      if have pacman; then PKG=msys2
      elif have choco; then PKG=choco
      elif have scoop; then PKG=scoop
      elif have winget; then PKG=winget
      fi
      ;;
    *)
      # Fall back: some environments report differently
      if [[ -n "${WINDIR:-}" ]] || [[ -n "${OS:-}" && "${OS}" == "Windows_NT" ]]; then
        OS_FAMILY=windows
        if have pacman; then PKG=msys2
        elif have choco; then PKG=choco
        elif have scoop; then PKG=scoop
        elif have winget; then PKG=winget
        fi
      elif [[ "$(uname -o 2>/dev/null || true)" == "GNU/Linux" ]]; then
        OS_FAMILY=linux
      fi
      ;;
  esac
}

detect_os

info "Host: ${OS_FAMILY} ($(uname -sm 2>/dev/null || echo unknown)), package manager: ${PKG}"
info "Repository root: ${REPO_ROOT}"

# ---------------------------------------------------------------------------
# Privilege helper
# ---------------------------------------------------------------------------
run_root() {
  if [[ "$(id -u 2>/dev/null || echo 1)" == "0" ]]; then
    "$@"
  elif have sudo; then
    if [[ "${NONINTERACTIVE}" == "1" ]]; then
      sudo -n "$@"
    else
      sudo "$@"
    fi
  else
    err "Need root privileges to run: $*"
    err "Re-run as root or install sudo."
    return 1
  fi
}

confirm_install() {
  local what="$1"
  if [[ "${NONINTERACTIVE}" == "1" ]]; then
    return 0
  fi
  if [[ ! -t 0 ]]; then
    return 0
  fi
  printf '%sInstall %s? [Y/n] %s' "${C_BOLD}" "${what}" "${C_RESET}"
  read -r ans || ans=y
  case "${ans}" in
    ""|y|Y|yes|YES) return 0 ;;
    *) return 1 ;;
  esac
}

# ---------------------------------------------------------------------------
# Package install primitives
# ---------------------------------------------------------------------------
pkg_install() {
  # pkg_install <human-name> <pkg1> [pkg2...]
  local name="$1"; shift
  local pkgs=("$@")
  if [[ ${#pkgs[@]} -eq 0 ]]; then
    warn "No package mapping for ${name} on ${OS_FAMILY}/${PKG}"
    return 1
  fi
  if ! confirm_install "${name} (${pkgs[*]})"; then
    warn "Skipped ${name}"
    return 1
  fi
  info "Installing ${name} via ${PKG}: ${pkgs[*]}"
  case "${PKG}" in
    brew)
      brew install "${pkgs[@]}"
      ;;
    apt)
      run_root apt-get update -y
      run_root DEBIAN_FRONTEND=noninteractive apt-get install -y "${pkgs[@]}"
      ;;
    dnf)
      run_root dnf install -y "${pkgs[@]}"
      ;;
    yum)
      run_root yum install -y "${pkgs[@]}"
      ;;
    pacman)
      run_root pacman -Sy --noconfirm "${pkgs[@]}"
      ;;
    msys2)
      # MSYS2 pacman usually does not need sudo
      pacman -Sy --noconfirm "${pkgs[@]}"
      ;;
    zypper)
      run_root zypper --non-interactive install "${pkgs[@]}"
      ;;
    apk)
      run_root apk add --no-cache "${pkgs[@]}"
      ;;
    choco)
      choco install -y "${pkgs[@]}"
      ;;
    scoop)
      scoop install "${pkgs[@]}"
      ;;
    winget)
      for p in "${pkgs[@]}"; do
        winget install --id "${p}" -e --accept-source-agreements --accept-package-agreements || true
      done
      ;;
    none)
      err "No supported package manager found for ${name}."
      return 1
      ;;
    *)
      err "Unknown package manager: ${PKG}"
      return 1
      ;;
  esac
}

# Per-OS package name maps
pkgs_for_git() {
  case "${PKG}" in
    brew) echo "git" ;;
    apt) echo "git" ;;
    dnf|yum) echo "git" ;;
    pacman|msys2) echo "git" ;;
    zypper) echo "git" ;;
    apk) echo "git" ;;
    choco) echo "git" ;;
    scoop) echo "git" ;;
    winget) echo "Git.Git" ;;
    *) echo "git" ;;
  esac
}

pkgs_for_python() {
  case "${PKG}" in
    brew) echo "python" ;;
    apt) echo "python3 python3-pip python3-venv" ;;
    dnf|yum) echo "python3 python3-pip" ;;
    pacman) echo "python python-pip" ;;
    msys2) echo "python python-pip" ;;
    zypper) echo "python3 python3-pip" ;;
    apk) echo "python3 py3-pip" ;;
    choco) echo "python" ;;
    scoop) echo "python" ;;
    winget) echo "Python.Python.3.12" ;;
    *) echo "python3" ;;
  esac
}

pkgs_for_build_tools() {
  # C toolchain for crates that compile C (e.g. bundled sqlite via rusqlite)
  case "${PKG}" in
    brew) echo "" ;;  # Xcode CLT handled separately
    apt) echo "build-essential pkg-config" ;;
    dnf|yum) echo "gcc make pkgconf" ;;
    pacman) echo "base-devel" ;;
    msys2) echo "mingw-w64-x86_64-gcc mingw-w64-x86_64-pkg-config make" ;;
    zypper) echo "gcc make pkg-config" ;;
    apk) echo "build-base pkgconf" ;;
    choco) echo "" ;;  # Visual Studio Build Tools — too heavy; document only
    scoop) echo "" ;;
    winget) echo "" ;;
    *) echo "" ;;
  esac
}

pkgs_for_tectonic() {
  case "${PKG}" in
    brew) echo "tectonic" ;;
    apt) echo "tectonic" ;;  # may be missing on older Ubuntu; handle failure
    dnf|yum) echo "tectonic" ;;
    pacman) echo "tectonic" ;;
    msys2) echo "" ;;  # often not packaged; cargo install fallback
    zypper) echo "tectonic" ;;
    apk) echo "" ;;
    choco) echo "" ;;
    scoop) echo "" ;;
    winget) echo "" ;;
    *) echo "" ;;
  esac
}

pkgs_for_latex_fallback() {
  # Heavier full TeX stacks when tectonic is unavailable
  case "${PKG}" in
    brew) echo "basictex" ;;
    apt) echo "texlive-latex-base latexmk" ;;
    dnf|yum) echo "texlive-scheme-basic latexmk" ;;
    pacman) echo "texlive-basic texlive-bin latexmk" ;;
    msys2) echo "mingw-w64-x86_64-texlive-core" ;;
    zypper) echo "texlive-latex latexmk" ;;
    apk) echo "texmf-dist texlive" ;;
    choco) echo "miktex" ;;
    scoop) echo "latex" ;;
    winget) echo "MiKTeX.MiKTeX" ;;
    *) echo "" ;;
  esac
}

# ---------------------------------------------------------------------------
# Dependency: Git
# ---------------------------------------------------------------------------
ensure_git() {
  if have git; then
    ok "$(git --version 2>/dev/null | head -1)"
    return 0
  fi
  if [[ "${CHECK_ONLY}" == "1" ]]; then
    warn "git: missing (required for sil init/status/log/commit proposals)"
    return 1
  fi
  info "git is required (history + Sci-Action trailers)"
  # shellcheck disable=SC2046
  pkg_install "git" $(pkgs_for_git) || true
  if have git; then
    ok "git installed"
    return 0
  fi
  err "git still not on PATH. Install Git manually: https://git-scm.com/downloads"
  return 1
}

# ---------------------------------------------------------------------------
# Dependency: Rust / cargo
# ---------------------------------------------------------------------------
ensure_rust() {
  if have cargo && have rustc; then
    ok "rustc $(rustc --version 2>/dev/null | head -1)"
    ok "cargo $(cargo --version 2>/dev/null | head -1)"
    return 0
  fi
  if [[ "${CHECK_ONLY}" == "1" ]]; then
    warn "rustc/cargo: missing (required to compile sil; needs recent stable with edition 2024)"
    return 1
  fi
  info "Installing Rust via rustup (https://rustup.rs)"
  if ! confirm_install "Rust toolchain (rustup)"; then
    warn "Skipped Rust install"
    return 1
  fi
  case "${OS_FAMILY}" in
    windows)
      if have rustup-init; then
        rustup-init -y --default-toolchain stable
      elif have curl; then
        # Prefer the Windows rustup if available via curl in Git Bash
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
      else
        err "Install Rust from https://rustup.rs (rustup-init.exe) and re-run."
        return 1
      fi
      # shellcheck disable=SC1091
      [[ -f "${USERPROFILE:-$HOME}/.cargo/env" ]] && . "${USERPROFILE:-$HOME}/.cargo/env" || true
      [[ -f "${HOME}/.cargo/env" ]] && . "${HOME}/.cargo/env" || true
      ;;
    *)
      if have curl; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
      elif have wget; then
        wget -qO- https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
      else
        err "Need curl or wget to install rustup."
        return 1
      fi
      # shellcheck disable=SC1091
      [[ -f "${HOME}/.cargo/env" ]] && . "${HOME}/.cargo/env"
      ;;
  esac
  if have cargo && have rustc; then
    ok "Rust installed: $(rustc --version)"
    return 0
  fi
  err "Rust install finished but cargo/rustc not on PATH. Open a new shell or source \$HOME/.cargo/env"
  return 1
}

# ---------------------------------------------------------------------------
# Dependency: C build tools (for bundled sqlite / native crates)
# ---------------------------------------------------------------------------
ensure_build_tools() {
  local need=0
  case "${OS_FAMILY}" in
    macos)
      if ! xcode-select -p >/dev/null 2>&1; then
        need=1
      fi
      ;;
    linux)
      if ! have cc && ! have gcc && ! have clang; then
        need=1
      fi
      ;;
    windows)
      # MSYS2/MinGW gcc or MSVC; only auto-install on msys2
      if [[ "${PKG}" == "msys2" ]] && ! have gcc; then
        need=1
      fi
      ;;
  esac

  if [[ "${need}" == "0" ]]; then
    if have cc || have gcc || have clang || have cl; then
      ok "C compiler present ($(command -v cc 2>/dev/null || command -v gcc 2>/dev/null || command -v clang 2>/dev/null || command -v cl 2>/dev/null || echo found))"
    else
      ok "build tools: assumed present or not required yet"
    fi
    return 0
  fi

  if [[ "${CHECK_ONLY}" == "1" ]]; then
    warn "C build tools: missing (needed to compile bundled SQLite / native crates)"
    return 1
  fi

  case "${OS_FAMILY}" in
    macos)
      info "Installing Xcode Command Line Tools (compiler + headers)"
      if confirm_install "Xcode Command Line Tools"; then
        xcode-select --install 2>/dev/null || true
        warn "If a GUI installer opened, finish it, then re-run this script."
      fi
      ;;
    *)
      local pkgs
      pkgs="$(pkgs_for_build_tools)"
      if [[ -n "${pkgs}" ]]; then
        # shellcheck disable=SC2086
        pkg_install "C build tools" ${pkgs} || true
      else
        warn "Install a C toolchain manually (Visual Studio Build Tools on Windows, or MSYS2 mingw-w64-gcc)."
      fi
      ;;
  esac
}

# ---------------------------------------------------------------------------
# Dependency: Python 3 + pip packages
# ---------------------------------------------------------------------------
resolve_python() {
  if [[ -n "${SIL_PYTHON:-}" ]] && have "${SIL_PYTHON}"; then
    echo "${SIL_PYTHON}"
    return 0
  fi
  if have python3; then
    echo "python3"
    return 0
  fi
  if have python; then
    # Ensure it is Python 3
    if python -c 'import sys; raise SystemExit(0 if sys.version_info[0] >= 3 else 1)' 2>/dev/null; then
      echo "python"
      return 0
    fi
  fi
  return 1
}

ensure_python() {
  local py
  if py="$(resolve_python)"; then
    ok "python $($py --version 2>&1) [$py]"
    PYTHON_BIN="$py"
    return 0
  fi
  if [[ "${CHECK_ONLY}" == "1" ]]; then
    warn "python3: missing (required for sil parse / sil source fetch helpers)"
    return 1
  fi
  info "Python 3 is required for python/parse_with_marker.py and python/download_pdf.py"
  # shellcheck disable=SC2046
  pkg_install "Python 3" $(pkgs_for_python) || true
  if py="$(resolve_python)"; then
    ok "python installed: $($py --version 2>&1)"
    PYTHON_BIN="$py"
    return 0
  fi
  err "Python 3 still not found. Install from https://www.python.org/downloads/"
  return 1
}

pip_install() {
  local py="$1"; shift
  # Prefer python -m pip; ensure pip exists
  if ! "$py" -m pip --version >/dev/null 2>&1; then
    if have "$py"; then
      "$py" -m ensurepip --upgrade 2>/dev/null || true
    fi
  fi
  if ! "$py" -m pip --version >/dev/null 2>&1; then
    err "pip not available for $py"
    return 1
  fi
  "$py" -m pip install --user --upgrade "$@"
}

ensure_python_packages() {
  local py="${PYTHON_BIN:-}"
  if [[ -z "${py}" ]]; then
    if ! py="$(resolve_python)"; then
      return 1
    fi
    PYTHON_BIN="$py"
  fi

  local req="${REPO_ROOT}/python/requirements.txt"
  if [[ ! -f "${req}" ]]; then
    warn "python/requirements.txt not found; skipping pip packages"
    return 0
  fi

  if [[ "${CHECK_ONLY}" == "1" ]]; then
    if "$py" -c 'import pypdf' 2>/dev/null; then
      ok "pypdf: installed"
    else
      warn "pypdf: missing (fallback PDF text extraction; see python/requirements.txt)"
    fi
    if "$py" -c 'import marker' 2>/dev/null; then
      ok "marker: installed"
    else
      dim "marker-pdf: not installed (optional; better parse quality with --with-marker)"
    fi
    return 0
  fi

  info "Installing Python packages from python/requirements.txt (pypdf, …)"
  if confirm_install "Python packages from requirements.txt"; then
    pip_install "$py" -r "${req}" || warn "pip install -r requirements.txt had errors"
  fi

  if [[ "${WITH_MARKER}" == "1" ]]; then
    info "Installing marker-pdf (large ML stack; may take several minutes)"
    if confirm_install "marker-pdf"; then
      # Package name on PyPI is marker-pdf; import name is marker
      if ! pip_install "$py" "marker-pdf"; then
        warn "marker-pdf install failed. sil parse still works with pypdf fallback."
      else
        ok "marker-pdf installed"
      fi
    fi
  else
    dim "Skipping marker-pdf (pass --with-marker for high-quality PDF parsing)"
  fi
}

# ---------------------------------------------------------------------------
# Dependency: LaTeX engine (optional, for sil build)
# ---------------------------------------------------------------------------
have_latex_engine() {
  have tectonic || have latexmk || have pdflatex || have xelatex || have lualatex
}

ensure_latex() {
  if have_latex_engine; then
    local eng
    for eng in tectonic latexmk pdflatex xelatex lualatex; do
      if have "$eng"; then
        ok "LaTeX engine: $eng"
        return 0
      fi
    done
  fi

  if [[ "${WITH_LATEX}" != "1" ]]; then
    dim "LaTeX engine: not installed (optional; pass --with-latex for sil build)"
    return 0
  fi

  if [[ "${CHECK_ONLY}" == "1" ]]; then
    warn "LaTeX engine: missing (needed for sil build; default config uses tectonic)"
    return 1
  fi

  info "Installing LaTeX engine (prefer tectonic)"
  local tpkgs
  tpkgs="$(pkgs_for_tectonic)"
  if [[ -n "${tpkgs}" ]]; then
    # shellcheck disable=SC2086
    if pkg_install "tectonic" ${tpkgs}; then
      if have tectonic; then
        ok "tectonic installed"
        return 0
      fi
    fi
  fi

  # Cargo fallback for tectonic (works on many platforms)
  if have cargo; then
    info "Trying cargo install tectonic"
    if confirm_install "tectonic via cargo"; then
      if cargo install tectonic; then
        ok "tectonic installed via cargo"
        return 0
      fi
    fi
  fi

  local fpkgs
  fpkgs="$(pkgs_for_latex_fallback)"
  if [[ -n "${fpkgs}" ]]; then
    # shellcheck disable=SC2086
    pkg_install "TeX distribution / latexmk" ${fpkgs} || true
  fi

  if have_latex_engine; then
    ok "LaTeX engine available"
    return 0
  fi
  warn "No LaTeX engine found. Install tectonic or a TeX Live/MiKTeX stack for sil build."
  return 1
}

# ---------------------------------------------------------------------------
# Build / install sil
# ---------------------------------------------------------------------------
build_sil() {
  if [[ "${SKIP_BUILD}" == "1" ]]; then
    dim "Skipping sil compile (--skip-build)"
    return 0
  fi
  if [[ "${CHECK_ONLY}" == "1" ]]; then
    if have sil; then
      ok "sil on PATH: $(command -v sil)"
    else
      dim "sil binary: not on PATH (build with this script without --check-only)"
    fi
    return 0
  fi

  if ! have cargo; then
    err "cargo required to build sil"
    return 1
  fi

  info "Building and installing sil from ${REPO_ROOT}"
  (
    cd "${REPO_ROOT}"
    cargo install --path crates/sil --force
  )
  if have sil; then
    ok "sil installed: $(command -v sil)"
    sil --help >/dev/null 2>&1 || true
  else
    warn "cargo install finished; ensure \$HOME/.cargo/bin is on your PATH"
    dim '  export PATH="$HOME/.cargo/bin:$PATH"'
  fi
}

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
print_summary() {
  echo
  info "Dependency summary"
  local status=0
  if have git; then ok "git"; else warn "git MISSING"; status=1; fi
  if have rustc && have cargo; then ok "rust/cargo"; else warn "rust/cargo MISSING"; status=1; fi
  if resolve_python >/dev/null 2>&1; then ok "python3"; else warn "python3 MISSING"; status=1; fi
  if resolve_python >/dev/null 2>&1 && "$(resolve_python)" -c 'import pypdf' 2>/dev/null; then
    ok "pypdf"
  else
    dim "pypdf optional-but-recommended"
  fi
  if resolve_python >/dev/null 2>&1 && "$(resolve_python)" -c 'import marker' 2>/dev/null; then
    ok "marker"
  else
    dim "marker optional (--with-marker)"
  fi
  if have_latex_engine; then
    ok "latex engine"
  else
    dim "latex optional (--with-latex)"
  fi
  if have sil; then ok "sil"; else dim "sil not on PATH yet"; fi
  return "${status}"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  local failed=0

  ensure_git || failed=1
  ensure_build_tools || true
  ensure_rust || failed=1
  ensure_python || failed=1
  ensure_python_packages || true
  ensure_latex || true
  build_sil || failed=1

  print_summary || true

  echo
  if [[ "${CHECK_ONLY}" == "1" ]]; then
    if [[ "${failed}" -ne 0 ]]; then
      warn "Some required dependencies are missing."
      exit 1
    fi
    ok "Check complete."
    exit 0
  fi

  if [[ "${failed}" -ne 0 ]]; then
    err "Finished with missing required dependencies. Fix the items above and re-run."
    exit 1
  fi

  cat <<EOF

${C_GREEN}${C_BOLD}Done.${C_RESET} Next steps:

  # Ensure cargo bin is on PATH (if needed)
  export PATH="\$HOME/.cargo/bin:\$PATH"

  # Create a project
  sil init my-paper
  cd my-paper

  # Optional env for Python helpers
  # export SIL_PYTHON=python3
  # export SIL_PARSE_SCRIPT=${REPO_ROOT}/python/parse_with_marker.py

EOF
}

main "$@"
