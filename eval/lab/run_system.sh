#!/usr/bin/env bash
# Run one lab system in isolation: start it fresh, measure it, always remove it.
# Usage: run_system.sh SYSTEM_ID RUN_DIRECTORY
#
# Requires RRFLOW_LAB_BIN (rrflow-lab), and for RRFlow arms RRFLOW_BIN (rrflow).
# Set RRFLOW_LAB_CPUSET to give every system the same cores, RRFLOW_LAB_CLIENT_CPUSET to
# keep the harness off them, and RRFLOW_LAB_MANIFEST to choose another test. Set
# RRFLOW_LAB_VDBBENCH to the vectordbbench command to drive qdrant, milvus or lance-hnsw-sq
# with that harness instead, as a cross-check on this one. Set RRFLOW_LAB_DEFINITIONS to a folder
# from `rrflow-lab plan --out` to read systems from it, as run_all.sh does, and each run then
# records which pack it came from. Set RRFLOW_LAB_POD to a container this script runs inside:
# each system container then joins its network, so every endpoint is this container's own
# loopback and nothing is published on the host, and its data goes to a named volume the run
# creates and removes. No value here is a secret:
# SurrealDB's password is generated per run and exists only in this process's environment.
set -euo pipefail
system=$1
run_directory=$(mkdir -p "$2" && cd "$2" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
repository=$(cd "$here/../.." && pwd)
manifest=${RRFLOW_LAB_MANIFEST:-$here/vector-small.json}
cpuset=${RRFLOW_LAB_CPUSET:-}
pod=${RRFLOW_LAB_POD:-}
: "${RRFLOW_LAB_BIN:?set RRFLOW_LAB_BIN to the rrflow-lab binary}"

# Keep TMPDIR on the disk under test: a RAM-backed /tmp would flatter whatever runs there.
work=$(mktemp -d "${TMPDIR:-/tmp}/rrflow-lab-XXXXXX")
export RRFLOW_LAB_ROOT=$work
export RRFLOW_LAB_HELPER_IMAGE=qdrant/qdrant@sha256:9bfc89f9914fc14876a88c05270685921a55b8877b254a98bcc1cc2f0a424cd2
mkdir -p "$work/data"
# Some images run as a non-root user, and this directory is private to the run.
chmod 0777 "$work/data"
container=
volume=
background=
sampler=
cleanup() {
  if [ -n "$sampler" ]; then
    kill "$sampler" 2> /dev/null || true
  fi
  if [ -n "$container" ]; then
    docker logs --tail 2000 "$container" > "$run_directory/$system.server.log" 2>&1 || true
    docker rm -f -v "$container" > /dev/null 2>&1 || true
  fi
  if [ -n "$volume" ]; then
    docker volume rm "$volume" > /dev/null 2>&1 || true
  fi
  if [ -n "$background" ]; then
    kill "$background" 2> /dev/null || true
    wait "$background" 2> /dev/null || true
  fi
  # Container users can own these files, so a container removes them. In a pod no container
  # wrote here, and this folder is not one the Docker host can see.
  if [ -z "$pod" ]; then
    docker run --rm -v "$work:/work" --entrypoint sh "$RRFLOW_LAB_HELPER_IMAGE" \
      -c 'rm -rf /work/data' > /dev/null 2>&1 || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT
trap 'exit 143' TERM INT

wait_http() {
  for _ in $(seq 1 "$2"); do
    curl -s -o /dev/null --max-time 2 "$1" && return 0
    sleep 1
  done
  echo "$1 did not answer" >&2
  return 1
}

# An array, not a function: `function &` would background a subshell, whose process is
# not the server, so memory probes and cleanup would both act on the wrong process.
pinned=()
docker_cpus=()
if [ -n "$cpuset" ]; then
  pinned=(taskset -c "$cpuset")
  docker_cpus=(--cpuset-cpus "$cpuset")
fi

# A leftover server would answer in place of the one about to start and be measured.
require_port_free() {
  if curl -s -o /dev/null --max-time 1 "http://127.0.0.1:$1/"; then
    echo "port $1 is already answering; refusing to measure a stale server" >&2
    exit 5
  fi
}

start_rrflow() {
  : "${RRFLOW_BIN:?set RRFLOW_BIN to the rrflow binary}"
  mkdir -p "$work/project" "$work/bin"
  # Installation refuses a hard-linked executable, and cargo hard-links its outputs.
  install -m 755 "$RRFLOW_BIN" "$work/bin/rrflow"
  local rrflow=$work/bin/rrflow
  identity=$("$rrflow" install identity --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["installation_identity"])')
  configuration=$(python3 "$here/rrflow_settings.py" "$manifest" "$system"     "$repository/crates/authority/rrd-engine/assets/install/default-engram-configuration-v3.toml" "$work/engram.toml")
  "$rrflow" install plan --project "$work/project" --identity "$identity" --mode fresh     ${configuration:+--configuration "$configuration"} --json > "$work/plan.json"
  digest=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["installation"]["plan_sha256"])' "$work/plan.json")
  "$rrflow" install apply --project "$work/project" --mode fresh --plan "$work/plan.json" --expect "$digest" > /dev/null
  "${pinned[@]}" "$rrflow" serve --project "$work/project" --bind 127.0.0.1:19477     > "$run_directory/$system.engine.log" 2>&1 &
  background=$!
  echo "$background" > "$work/rrflow.pid"
  wait_http http://127.0.0.1:19477/v1/capabilities 120
  # The credential goes straight into this process's environment. It is never printed.
  eval "$(python3 "$here/rrflow_credentials.py" "$work/project")"
}

definitions=${RRFLOW_LAB_DEFINITIONS:-$here}
definition=$definitions/systems/$system.json
pack_notes=()
if [ -f "$definitions/definitions.env" ]; then
  while IFS='=' read -r key value; do
    pack_notes+=(--note "definitions_$key=$value")
  done < "$definitions/definitions.env"
fi
if [ ! -f "$definition" ]; then
  echo "no $definition: a new system is one file there, plus a shim if it needs one" >&2
  exit 2
fi
declare -A launch=()
"$RRFLOW_LAB_BIN" launch-info --system "$definition" > "$work/launch.info"
while IFS='=' read -r key value; do
  launch[$key]=$value
done < "$work/launch.info"
if [ "${launch[kind]}" = none ]; then
  echo "$system is a declared gap, so there is nothing to run. Its reason appears in the report."
  exit 0
fi
for port in ${launch[ports]:-}; do
  require_port_free "$port"
done

wait_ready() {
  local contains=${launch[ready_contains]:-}
  for _ in $(seq 1 "${launch[ready_seconds]}"); do
    if [ -n "$contains" ]; then
      curl -s --max-time 2 "${launch[ready_url]}" | grep -q "$contains" && return 0
    else
      curl -s -o /dev/null --max-time 2 "${launch[ready_url]}" && return 0
    fi
    sleep 1
  done
  echo "${launch[ready_url]} did not answer" >&2
  return 1
}

# Words, separated by NUL, so no value here is ever read as shell.
launch_words() {
  "$RRFLOW_LAB_BIN" launch-args --system "$definition" --data "$work/data" --lab "$here" \
    ${cpuset:+--cpuset "$cpuset"} "$@" > "$work/launch.args"
  mapfile -d '' -t words < "$work/launch.args"
}

start_container() {
  local pair secret outside
  for pair in ${launch[host_env]:-}; do
    export "${pair?}"
  done
  # A secret is generated for this run, and reaches the container by name, off the process list.
  for pair in ${launch[secrets]:-}; do
    secret=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
    export "${pair%%=*}=$secret" "${pair#*=}=$secret"
  done
  for pair in ${launch[env_from_host]:-}; do
    outside=${pair#*=}
    export "${pair%%=*}=${!outside}"
  done
  if [ -n "$pod" ]; then
    volume=$(docker volume create "$pod-$system-${work##*-}")
    export RRFLOW_LAB_VOLUME=$volume
    launch_words --pod "$pod" --volume "$volume"
  else
    launch_words
  fi
  # Ownership is the ID Docker returns, never the name in the definition: if the
  # name is already in use the run fails here and cleanup removes nothing.
  local created
  created=$(docker run -d "${words[@]}") || return $?
  if [[ ! $created =~ ^[0-9a-f]{64}$ ]]; then
    echo "Docker did not return a valid container ID; refusing name-based cleanup" >&2
    return 1
  fi
  container=$created
  # Probes measure this container, never another run's with the same name.
  export RRFLOW_LAB_CONTAINER=$container
  wait_ready
}

start_process() {
  launch_words
  "${pinned[@]}" "${words[@]}" &
  background=$!
  if [ -n "${launch[pid_file]:-}" ]; then
    echo "$background" > "$work/${launch[pid_file]}"
  fi
  wait_ready
}

case "${launch[kind]}" in
  rrflow_engine) start_rrflow ;;
  container) start_container ;;
  process)
    # A harness that embeds the system in its own process needs no server from here.
    if [ -z "${RRFLOW_LAB_VDBBENCH:-}" ] || [ "${launch[embedded_under_vectordbbench]:-}" != true ]; then
      start_process
    fi
    ;;
esac

status=0
client_cpuset=${RRFLOW_LAB_CLIENT_CPUSET:-}
client=()
[ -n "$client_cpuset" ] && client=(taskset -c "$client_cpuset")
host_samples=$run_directory/$system.host.csv
"${client[@]}" "$here/sample_host.sh" > "$host_samples" &
sampler=$!
if [ -n "${RRFLOW_LAB_VDBBENCH:-}" ]; then
  # Its dataset stays between runs, and each result is kept beside the run for rrflow-lab import.
  ef=${RRFLOW_LAB_VDBBENCH_EF:-128}
  "$RRFLOW_LAB_BIN" vdbbench-args --system "$definition" --data "$work/data" --ef "$ef" > "$work/driver.args"
  mapfile -d '' -t driver < "$work/driver.args"
  results=$work/vdbbench-results
  mkdir -p "$results"
  DATASET_LOCAL_DIR=${RRFLOW_LAB_VDBBENCH_DATASETS:-$HOME/lab-vdbbench-datasets} RESULTS_LOCAL_DIR=$results \
    "${client[@]}" "$RRFLOW_LAB_VDBBENCH" "${driver[@]}" --case-type "${RRFLOW_LAB_VDBBENCH_CASE:?name a VectorDBBench case}" \
    --k 10 --num-concurrency 1,8 --concurrency-duration 30 --db-label "lab-$system-ef-$ef" || status=$?
  find "$results" -name 'result_*.json' -exec cp {} "$run_directory/$system.ef-$ef.vdbbench.json" \;
else
  (cd "$repository" && "${client[@]}" "$RRFLOW_LAB_BIN" run --manifest "$manifest" --system "$system" \
    --out "$run_directory" --note "server_cpuset=${cpuset:-unpinned}" \
    --note "client_cpuset=${client_cpuset:-unpinned}" "${pack_notes[@]}") || status=$?
fi
# Another workload can slow a run without touching its cores, so a busy GPU is reported.
awk -F, 'NR > 1 && $4 != "" {
    samples++
    if ($4 > 5) busy++
    if ($4 > peak) peak = $4
  }
  END {
    if (busy) printf "warning: GPU busy in %d of %d host samples, peak %d%%; other work may have slowed this run\n", busy, samples, peak
  }' "$host_samples" >&2
exit "$status"
