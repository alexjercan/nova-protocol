# Shared helpers for the local web dev servers (scripts/serve-web.sh and
# scripts/serve-mods.sh). Sourced, not executed - no shebang, not executable.
#
# Why random ports: several worktrees (and several agents) run these servers at
# once. Fixed ports mean the second one dies on EADDRINUSE, or worse, silently
# talks to the first one's build. Everything here allocates in 7000-7999 so the
# whole preview stack is recognisable at a glance and never collides with the
# documented standalone defaults (trunk :8080, preview-web.sh :8090).

NOVA_PORT_LO=7000
NOVA_PORT_HI=7999

# Print a free TCP port in NOVA_PORT_LO..NOVA_PORT_HI, chosen at random so two
# scripts starting in the same second do not race onto the same candidate.
# There is an unavoidable window between this bind test and the server's own
# bind; the callers start immediately, which keeps it negligible.
nova_free_port() {
    python3 - "$NOVA_PORT_LO" "$NOVA_PORT_HI" <<'PY'
import random
import socket
import sys

lo, hi = int(sys.argv[1]), int(sys.argv[2])
for port in random.sample(range(lo, hi + 1), hi - lo + 1):
    sock = socket.socket()
    try:
        sock.bind(("127.0.0.1", port))
    except OSError:
        continue
    finally:
        sock.close()
    print(port)
    sys.exit(0)
sys.exit("no free port in {}-{}".format(lo, hi))
PY
}

# nova_resolve_port VAR - echo $VAR if it is already set to a valid port,
# otherwise allocate a free one. Lets a caller pin any single port
# (NOVA_UI_PORT=7100 scripts/serve-web.sh) without giving up the rest.
nova_resolve_port() {
    local name="$1"
    local preset="${!name:-}"
    if [[ -n "$preset" ]]; then
        if [[ ! "$preset" =~ ^[0-9]+$ ]] || ((preset < 1 || preset > 65535)); then
            echo "$name is not a valid port: $preset" >&2
            return 1
        fi
        echo "$preset"
        return 0
    fi
    nova_free_port
}

# nova_wait_for_port PORT TIMEOUT_SECONDS - block until something accepts on
# 127.0.0.1:PORT. Used to hold the "here are your URLs" banner back until the
# servers can actually answer, so the printed links are never dead on arrival.
# Returns 1 on timeout; callers decide whether that is fatal.
nova_wait_for_port() {
    python3 - "$1" "$2" <<'PY'
import socket
import sys
import time

port, timeout = int(sys.argv[1]), float(sys.argv[2])
deadline = time.monotonic() + timeout
while time.monotonic() < deadline:
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=0.5):
            sys.exit(0)
    except OSError:
        time.sleep(0.25)
sys.exit(1)
PY
}
