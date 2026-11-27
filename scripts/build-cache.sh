# Sourced by scripts/gui.sh and lab/build.sh (not run). Host folder for the intermediate artifacts of the cargo builds in a
# container (deps, incremental, build scripts: Cargo's build-dir). The final binaries stay in CARGO_TARGET_DIR, inside each worktree.
#
#   build_cache_dir <gui-image|lab-build>
#     GALLERIA_BUILD_CACHE/<name> if the variable is set, else <git common dir>/galleria-build/<name>: the same folder for the
#     checkout and for all of its worktrees, so the dependencies are built once. Outside a git checkout: target/<name>-build.
#     The folder is created and its path printed.
build_cache_dir() {
  local dir common
  if [[ -n ${GALLERIA_BUILD_CACHE:-} ]]; then
    dir=$GALLERIA_BUILD_CACHE/$1
  elif common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null); then
    dir=$common/galleria-build/$1
  else
    dir=$PWD/target/$1-build
  fi
  mkdir -p -- "$dir"
  printf '%s\n' "$dir"
}
