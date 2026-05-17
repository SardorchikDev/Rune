#!/usr/bin/env bash
# ============================================================================
# Rune Installer
# ============================================================================
# One-shot installer for the Rune agent framework. Clones the repo, runs an
# interactive config wizard (JWT secret, hashed dashboard password, at least
# one LLM provider key), and brings up the full Docker stack (qdrant, backend,
# dashboard).
#
# Quick start:
#   curl -fsSL https://raw.githubusercontent.com/SardorchikDev/Rune/main/install.sh | bash
#
# With flags:
#   curl -fsSL .../install.sh | bash -s -- --dir ~/rune --skip-start
#
# Supported OS: Linux + macOS. Requires: bash 4+, git, docker, docker compose.
# ============================================================================

set -euo pipefail

# ---------- defaults ----------
REPO_URL_HTTPS="https://github.com/SardorchikDev/Rune.git"
DEFAULT_INSTALL_DIR="${RUNE_INSTALL_DIR:-$HOME/rune}"
DEFAULT_BRANCH="main"

INSTALL_DIR=""
BRANCH="$DEFAULT_BRANCH"
NON_INTERACTIVE=false
SKIP_CONFIG=false
SKIP_START=false
UPDATE=false
USE_BUILD=true

# ---------- ui helpers ----------
if [[ -t 1 ]]; then
    BOLD=$'\033[1m'
    RED=$'\033[38;2;255;59;48m'
    GREEN=$'\033[38;2;0;255;136m'
    AMBER=$'\033[38;2;255;187;51m'
    CYAN=$'\033[38;2;91;232;255m'
    MUTED=$'\033[38;2;125;133;144m'
    NC=$'\033[0m'
else
    BOLD=""; RED=""; GREEN=""; AMBER=""; CYAN=""; MUTED=""; NC=""
fi

log()    { printf '%s\n' "${CYAN}::${NC} $*"; }
ok()     { printf '%s\n' "${GREEN}::${NC} $*"; }
warn()   { printf '%s\n' "${AMBER}::${NC} $*"; }
err()    { printf '%s\n' "${RED}::${NC} $*" >&2; }
step()   { printf '\n%s\n' "${BOLD}${CYAN}==> $*${NC}"; }
muted()  { printf '%s\n' "${MUTED}   $*${NC}"; }

banner() {
    cat <<'BANNER'

  ╦═╗┬ ┬┌┐┌┌─┐
  ╠╦╝│ ││││├┤
  ╩╚═└─┘┘└┘└─┘   agent framework · v1.0
BANNER
    printf '\n'
}

usage() {
    cat <<USAGE
Usage: install.sh [options]

Options:
  --dir <path>          Install location (default: \$HOME/rune)
  --branch <name>       Git branch to track (default: main)
  --non-interactive     Use defaults / env vars; never prompt
  --skip-config         Don't run the config wizard
  --skip-start          Don't run \`docker compose up\` after install
  --update              Update an existing install (git pull only)
  --no-build            Pull pre-built images instead of building locally
  -h, --help            Show this help

Env vars (read in --non-interactive mode):
  RUNE_INSTALL_DIR              install path
  RUNE_DASHBOARD_PASSWORD       plaintext password (will be sha256-hashed)
  RUNE_DEFAULT_PROVIDER         gemini | groq | openrouter | fireworks | anthropic | openai | ollama
  RUNE_PROVIDER_API_KEY         api key for the default provider
  RUNE_TELEGRAM_BOT_TOKEN       optional, enables telegram bot
  RUNE_TELEGRAM_USER_IDS        comma-separated allow-list, e.g. "12345,67890"
USAGE
}

# ---------- arg parsing ----------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir)              INSTALL_DIR="$2"; shift 2 ;;
        --branch)           BRANCH="$2"; shift 2 ;;
        --non-interactive)  NON_INTERACTIVE=true; shift ;;
        --skip-config)      SKIP_CONFIG=true; shift ;;
        --skip-start)       SKIP_START=true; shift ;;
        --update)           UPDATE=true; shift ;;
        --no-build)         USE_BUILD=false; shift ;;
        -h|--help)          banner; usage; exit 0 ;;
        *)                  err "Unknown option: $1"; usage; exit 2 ;;
    esac
done

INSTALL_DIR="${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"

# When piped from curl, stdin is the script — open the TTY for prompts.
if [[ "$NON_INTERACTIVE" == false ]] && [[ ! -t 0 ]] && [[ -r /dev/tty ]]; then
    exec </dev/tty
fi
if [[ ! -t 0 ]]; then
    NON_INTERACTIVE=true
fi

# ---------- preflight ----------
detect_os() {
    case "$(uname -s)" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="macos" ;;
        *) err "Unsupported OS: $(uname -s) (Linux and macOS only)"; exit 1 ;;
    esac
    ok "Detected OS: $OS ($(uname -m))"
}

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "Missing required command: $1"
        muted "$2"
        return 1
    fi
}

detect_compose() {
    if docker compose version >/dev/null 2>&1; then
        COMPOSE=(docker compose)
        ok "Found docker compose plugin ($(docker compose version --short 2>/dev/null || echo unknown))"
    elif command -v docker-compose >/dev/null 2>&1; then
        COMPOSE=(docker-compose)
        warn "Using legacy docker-compose v1. v2 plugin is recommended."
    else
        err "Missing docker compose. Install Docker Engine 20.10+ with the compose plugin."
        muted "https://docs.docker.com/compose/install/"
        return 1
    fi
}

preflight() {
    step "Preflight"
    detect_os

    local missing=0
    require_cmd git "Install git first: https://git-scm.com/downloads" || missing=1
    require_cmd docker "Install Docker: https://docs.docker.com/get-docker/" || missing=1
    require_cmd openssl "Install openssl (most distros ship it by default)" || missing=1
    require_cmd sha256sum >/dev/null 2>&1 || \
        require_cmd shasum "Install coreutils (Linux) or use macOS shasum" || missing=1

    if ! docker info >/dev/null 2>&1; then
        err "Docker is installed but not running (\`docker info\` failed)."
        muted "Start Docker Desktop (macOS) or \`sudo systemctl start docker\` (Linux)."
        missing=1
    fi

    detect_compose || missing=1

    if [[ "$missing" != 0 ]]; then
        err "Fix the above and re-run."
        exit 1
    fi
    ok "All prerequisites satisfied."
}

# Cross-platform sha256 over stdin.
sha256_hex() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | awk '{print $1}'
    else
        shasum -a 256 | awk '{print $1}'
    fi
}

# ---------- clone / update ----------
clone_or_update() {
    if [[ -d "$INSTALL_DIR/.git" ]]; then
        if [[ "$UPDATE" == true ]]; then
            step "Updating $INSTALL_DIR (branch $BRANCH)"
            git -C "$INSTALL_DIR" fetch origin "$BRANCH"
            git -C "$INSTALL_DIR" checkout "$BRANCH"
            git -C "$INSTALL_DIR" pull --ff-only origin "$BRANCH"
        else
            warn "$INSTALL_DIR already exists. Skipping clone."
            muted "Re-run with --update to pull the latest from origin/$BRANCH."
        fi
    else
        step "Cloning Rune into $INSTALL_DIR"
        mkdir -p "$(dirname "$INSTALL_DIR")"
        git clone --branch "$BRANCH" --depth 1 "$REPO_URL_HTTPS" "$INSTALL_DIR"
    fi
    ok "Source ready at $INSTALL_DIR"
}

# ---------- config wizard ----------
prompt() {
    # prompt VAR "Question" "default"
    local __var="$1" question="$2" default="${3:-}" answer
    if [[ "$NON_INTERACTIVE" == true ]]; then
        printf -v "$__var" '%s' "${!__var:-$default}"
        return 0
    fi
    if [[ -n "$default" ]]; then
        read -r -p "${question} ${MUTED}[${default}]${NC} " answer || true
        answer="${answer:-$default}"
    else
        read -r -p "${question} " answer || true
    fi
    printf -v "$__var" '%s' "$answer"
}

prompt_secret() {
    local __var="$1" question="$2" answer
    if [[ "$NON_INTERACTIVE" == true ]]; then
        printf -v "$__var" '%s' "${!__var:-}"
        return 0
    fi
    read -r -s -p "${question} " answer || true
    printf '\n'
    printf -v "$__var" '%s' "$answer"
}

valid_provider() {
    case "$1" in
        gemini|groq|openrouter|fireworks|anthropic|openai|ollama) return 0 ;;
        *) return 1 ;;
    esac
}

# Maps provider key -> env var name in backend/.env.
provider_env_var() {
    case "$1" in
        gemini)     echo "GEMINI_API_KEY" ;;
        groq)       echo "GROQ_API_KEY" ;;
        openrouter) echo "OPENROUTER_API_KEY" ;;
        fireworks)  echo "FIREWORKS_API_KEY" ;;
        anthropic)  echo "ANTHROPIC_API_KEY" ;;
        openai)     echo "OPENAI_API_KEY" ;;
        ollama)     echo "" ;;  # local, no key
    esac
}

# Maps provider key -> sensible default model.
provider_default_model() {
    case "$1" in
        gemini)     echo "gemini-2.5-pro" ;;
        groq)       echo "llama3-70b-8192" ;;
        openrouter) echo "meta-llama/llama-3-70b-instruct" ;;
        fireworks)  echo "accounts/fireworks/models/llama-v3-70b-instruct" ;;
        anthropic)  echo "claude-sonnet-4-20250514" ;;
        openai)     echo "gpt-4o" ;;
        ollama)     echo "llama3" ;;
    esac
}

run_wizard() {
    step "Configuration"

    # JWT secret — always auto-generated unless one is already exported.
    local jwt_secret
    jwt_secret="${RUNE_JWT_SECRET:-$(openssl rand -hex 32)}"
    ok "Generated 64-char JWT secret"

    # Dashboard password.
    local password password_hash
    if [[ "$NON_INTERACTIVE" == true ]]; then
        password="${RUNE_DASHBOARD_PASSWORD:-}"
        if [[ -z "$password" ]]; then
            err "RUNE_DASHBOARD_PASSWORD must be set in --non-interactive mode."
            exit 1
        fi
    else
        while :; do
            prompt_secret password    "Dashboard password (min 8 chars):"
            if [[ ${#password} -lt 8 ]]; then
                warn "Password too short (need ≥8 chars)."
                continue
            fi
            local confirm
            prompt_secret confirm     "Confirm password:"
            if [[ "$password" != "$confirm" ]]; then
                warn "Passwords didn't match. Try again."
                continue
            fi
            break
        done
    fi
    password_hash="$(printf '%s' "$password" | sha256_hex)"
    ok "Hashed dashboard password (sha256)"

    # Default provider + key.
    local provider api_key default_model
    if [[ "$NON_INTERACTIVE" == true ]]; then
        provider="${RUNE_DEFAULT_PROVIDER:-gemini}"
        api_key="${RUNE_PROVIDER_API_KEY:-}"
        if ! valid_provider "$provider"; then
            err "Invalid RUNE_DEFAULT_PROVIDER: $provider"; exit 1
        fi
    else
        printf '\n%s\n' "${BOLD}Pick a default LLM provider:${NC}"
        printf '  1) gemini       (Google Gemini)\n'
        printf '  2) groq         (Groq)\n'
        printf '  3) openrouter   (OpenRouter)\n'
        printf '  4) fireworks    (Fireworks)\n'
        printf '  5) anthropic    (Claude)\n'
        printf '  6) openai       (OpenAI)\n'
        printf '  7) ollama       (local, no API key)\n'
        local choice
        prompt choice "Choice [1-7]:" "1"
        case "$choice" in
            1|gemini)     provider="gemini" ;;
            2|groq)       provider="groq" ;;
            3|openrouter) provider="openrouter" ;;
            4|fireworks)  provider="fireworks" ;;
            5|anthropic)  provider="anthropic" ;;
            6|openai)     provider="openai" ;;
            7|ollama)     provider="ollama" ;;
            *) warn "Unknown choice '$choice'. Defaulting to gemini."; provider="gemini" ;;
        esac
        if [[ "$provider" != "ollama" ]]; then
            prompt_secret api_key "$(provider_env_var "$provider") (leave blank to fill in later):"
        fi
    fi
    default_model="$(provider_default_model "$provider")"
    ok "Default provider: $provider · model: $default_model"
    if [[ "$provider" != "ollama" ]] && [[ -z "$api_key" ]]; then
        warn "No API key entered for $provider — Rune won't be able to call it until you fill it in."
    fi

    # Telegram (opt-in).
    local tg_token="" tg_ids="" tg_enabled="false"
    if [[ "$NON_INTERACTIVE" == true ]]; then
        tg_token="${RUNE_TELEGRAM_BOT_TOKEN:-}"
        tg_ids="${RUNE_TELEGRAM_USER_IDS:-}"
    else
        local enable_tg
        prompt enable_tg "Enable Telegram bot? [y/N]:" "n"
        if [[ "${enable_tg,,}" =~ ^y(es)?$ ]]; then
            prompt_secret tg_token "Telegram bot token (from @BotFather):"
            prompt tg_ids          "Comma-separated allowed user IDs (e.g. 12345,67890):"
        fi
    fi
    if [[ -n "$tg_token" ]] && [[ -n "$tg_ids" ]]; then
        tg_enabled="true"
        ok "Telegram bot enabled for $tg_ids"
    fi

    write_env_file       "$jwt_secret" "$password_hash" "$provider" "$api_key" "$tg_token"
    write_config_file    "$password_hash" "$provider" "$default_model" "$tg_ids" "$tg_enabled"
}

write_env_file() {
    local jwt="$1" pw_hash="$2" provider="$3" api_key="$4" tg_token="$5"
    local env_path="$INSTALL_DIR/backend/.env"
    if [[ -f "$env_path" ]]; then
        local backup
        backup="$env_path.bak.$(date +%s)"
        cp "$env_path" "$backup"
        warn "Existing $env_path backed up to $(basename "$backup")"
    fi
    {
        echo "# Generated by install.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "# Do NOT commit this file."
        echo ""
        echo "RUNE_JWT_SECRET=$jwt"
        echo "RUNE_DASHBOARD_PASSWORD_SHA256=$pw_hash"
        echo "RUST_LOG=rune=info,tower_http=warn"
        echo ""
        echo "DATABASE_URL=sqlite:///app/data/rune.db"
        echo ""
        echo "TELEGRAM_BOT_TOKEN=$tg_token"
        echo ""
        local key
        for key in GEMINI_API_KEY GROQ_API_KEY OPENROUTER_API_KEY FIREWORKS_API_KEY ANTHROPIC_API_KEY OPENAI_API_KEY; do
            if [[ "$(provider_env_var "$provider")" == "$key" ]]; then
                echo "$key=$api_key"
            else
                echo "$key="
            fi
        done
    } >"$env_path"
    chmod 600 "$env_path"
    ok "Wrote $env_path (mode 600)"
}

write_config_file() {
    local pw_hash="$1" provider="$2" model="$3" tg_ids="$4" tg_enabled="$5"
    local example="$INSTALL_DIR/backend/config.example.toml"
    local config="$INSTALL_DIR/backend/config.toml"
    if [[ ! -f "$example" ]]; then
        err "Missing $example — did the clone succeed?"
        exit 1
    fi
    if [[ -f "$config" ]]; then
        local backup
        backup="$config.bak.$(date +%s)"
        cp "$config" "$backup"
        warn "Existing $config backed up to $(basename "$backup")"
    fi

    # Build the allowed_user_ids array literal.
    local ids_literal="[]"
    if [[ -n "$tg_ids" ]]; then
        ids_literal="[${tg_ids// /}]"
    fi

    # Render via awk so we don't need a separate templating dep. We track which
    # TOML section we're in so substitutions only fire in the intended one
    # (e.g. `default_model` exists under [llm] AND every [llm.providers.*]).
    awk -v pw_hash="$pw_hash" \
        -v provider="$provider" \
        -v model="$model" \
        -v ids="$ids_literal" \
        -v tg_enabled="$tg_enabled" \
        '
        BEGIN { section = "" }
        /^\[[^]]+\]/ {
            section = $0
            sub(/^\[/, "", section)
            sub(/\]$/, "", section)
            print
            next
        }

        section == "server" && /^dashboard_password_sha256 *=/ {
            print "dashboard_password_sha256 = \"" pw_hash "\""; next
        }
        section == "llm" && /^default_provider *=/ {
            print "default_provider = \"" provider "\""; next
        }
        section == "llm" && /^default_model *=/ {
            print "default_model = \"" model "\""; next
        }

        section == "telegram" && /^allowed_user_ids *=/ {
            print "allowed_user_ids = " ids; next
        }
        section == "telegram" && /^enabled *=/ {
            print "enabled = " tg_enabled; next
        }

        { print }
        ' "$example" >"$config"

    chmod 600 "$config"
    ok "Wrote $config (mode 600)"
}

# ---------- launch ----------
launch_stack() {
    step "Launching Docker stack"
    cd "$INSTALL_DIR"

    local build_flag=()
    if [[ "$USE_BUILD" == true ]]; then
        build_flag=(--build)
    fi

    log "Running: ${COMPOSE[*]} up -d ${build_flag[*]}"
    "${COMPOSE[@]}" up -d "${build_flag[@]}"
    ok "Containers started."

    # Show short status.
    "${COMPOSE[@]}" ps
}

print_next_steps() {
    cat <<EOF

${BOLD}${GREEN}Rune is up.${NC}

  Dashboard:   ${CYAN}http://localhost:3000${NC}
  REST API:    ${CYAN}http://localhost:8080${NC}
  Qdrant:      ${CYAN}http://localhost:6333${NC}

  Login at ${CYAN}/login${NC} with the password you just set.

  ${BOLD}Useful commands (run from $INSTALL_DIR):${NC}
    ${MUTED}${COMPOSE[*]} logs -f backend     # follow agent logs${NC}
    ${MUTED}${COMPOSE[*]} restart backend     # restart after editing config${NC}
    ${MUTED}${COMPOSE[*]} down                # stop everything${NC}

  Config files (mode 600, do not commit):
    ${MUTED}$INSTALL_DIR/backend/.env${NC}
    ${MUTED}$INSTALL_DIR/backend/config.toml${NC}

  Need a different LLM provider? Edit ${MUTED}config.toml${NC} (\`default_provider\`)
  and add the matching key in ${MUTED}.env${NC}, then ${MUTED}${COMPOSE[*]} restart backend${NC}.
EOF
}

# ---------- main ----------
main() {
    banner
    preflight
    clone_or_update
    if [[ "$SKIP_CONFIG" == true ]]; then
        warn "Skipping config wizard (--skip-config)."
        warn "Copy backend/.env.example -> backend/.env and config.example.toml -> config.toml manually."
    else
        run_wizard
    fi
    if [[ "$SKIP_START" == true ]]; then
        ok "Install complete. Skipping start (--skip-start)."
        muted "Run \`(cd $INSTALL_DIR && ${COMPOSE[*]} up -d --build)\` when ready."
        exit 0
    fi
    launch_stack
    print_next_steps
}

main "$@"
