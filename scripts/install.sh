#!/bin/sh
# Bootstrap the manager, then use it to install the selected toolchain.
set -eu
LC_ALL=C
export LC_ALL

die() {
    printf 'rock installer: %s\n' "$*" >&2
    exit 1
}

[ "$#" -le 1 ] || die 'Usage: sh scripts/install.sh [vVERSION]'
channel=${1-stable}
case "$channel" in
    stable) release_path=latest/download ;;
    v*)
        [ "${#channel}" -le 128 ] || die 'Version is too long'
        printf '%s\n' "$channel" | grep -Eq '^v[0-9]+(\.[0-9]+)*([-+][A-Za-z0-9._+-]+)?$' ||
            die 'Expected a version such as v0.1.0'
        release_path=download/$channel
        ;;
    *) die 'Usage: sh scripts/install.sh [vVERSION] (default: stable)' ;;
esac

[ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ] ||
    die 'Only x86_64-unknown-linux-gnu is supported'
for tool in curl sha256sum tar gzip getconf awk mktemp chmod rm; do
    command -v "$tool" >/dev/null 2>&1 || die "Required command not found: $tool"
done
case "$(tar --version)" in
    *'GNU tar'*) ;;
    *) die 'GNU tar is required by rockup' ;;
esac
libc=$(getconf GNU_LIBC_VERSION 2>/dev/null) || die 'GNU glibc is required (not musl)'
printf '%s\n' "$libc" | awk '
    $1 == "glibc" { split($2, v, "."); if (v[1] > 2 || (v[1] == 2 && v[2] >= 39)) ok = 1 }
    END { exit !ok }
' || die 'Release binaries require glibc 2.39 or newer (Ubuntu 24.04 baseline)'

[ -n "${HOME-}" ] || die 'HOME must be set and nonempty'
ROCKUP_HOME=${ROCKUP_HOME-"$HOME/.rockup"}
case "$ROCKUP_HOME" in
    /*) ;;
    *) die 'ROCKUP_HOME must be a nonempty absolute path' ;;
esac
export ROCKUP_HOME
installed=$ROCKUP_HOME/bin/rockup
if [ -e "$installed" ] || [ -L "$installed" ]; then
    die "Refusing to overwrite $installed; use the existing rockup install $channel, rockup update, or rockup self update"
fi

umask 077
stage=$(mktemp -d "${TMPDIR:-/tmp}/rock-install.XXXXXXXXXX") || die 'Cannot create temporary directory'
trap 'rm -rf -- "$stage"' 0
trap 'exit 1' HUP INT TERM
asset=rockup-x86_64-unknown-linux-gnu
base=https://github.com/Rock-lang-org/Rock/releases/$release_path
for file in "$asset" "$asset.sha256"; do
    curl --disable --fail --silent --show-error --location \
        --proto '=https' --proto-redir '=https' \
        --retry 3 --connect-timeout 30 --max-time 1800 \
        --output "$stage/$file" "$base/$file" ||
        die "Download failed: $base/$file (a release with bootstrap assets must be published first)"
done

# Validate the sidecar before letting sha256sum interpret any filenames.
awk -v asset="$asset" '
    NR != 1 { bad = 1 }
    NR == 1 {
        hash = substr($0, 1, 64)
        separator = substr($0, 65, 2)
        if (length(hash) != 64 || hash ~ /[^0-9a-fA-F]/ ||
            (separator != "  " && separator != " *") || substr($0, 67) != asset) bad = 1
    }
    END { exit (NR != 1 || bad) }
' "$stage/$asset.sha256" || die 'Invalid SHA-256 sidecar: expected one line naming the exact rockup asset'
(cd "$stage" && sha256sum --check --strict "$asset.sha256") || die 'SHA-256 verification failed'
chmod 700 "$stage/$asset"
if [ -e "$installed" ] || [ -L "$installed" ]; then
    die "rockup appeared at $installed during download; refusing to overwrite it"
fi
"$stage/$asset" self install || die 'rockup manager installation failed'
[ -x "$installed" ] || die "rockup did not persist itself at $installed"
"$installed" install "$channel" ||
    die "toolchain installation failed; retry with: \"$installed\" install $channel"
printf '\nRock installed. Restart your shell, or run:\n  . "%s/env"\n\nThen try:\n  rock --version\n' "$ROCKUP_HOME"
