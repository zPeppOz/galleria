#!/usr/bin/env python3
"""Run Orca with a line-buffered debug file (lab/tests/gui-a11y.sh).

Orca writes every sentence it speaks to its debug file as a `SPEECH OUTPUT` line, but opens that file with the default
block buffering, so a test that reads the file while Orca runs sees nothing for a long time. This launcher makes Orca's
`open(<debug file>, 'w')` line-buffered and then runs /usr/bin/orca unchanged, with the same arguments.
"""
import builtins
import runpy
import sys

_real_open = builtins.open


def _line_buffered_open(file, mode="r", buffering=-1, *args, **kwargs):
    if isinstance(file, str) and "w" in mode and file.endswith(".log"):
        buffering = 1
    return _real_open(file, mode, buffering, *args, **kwargs)


builtins.open = _line_buffered_open
sys.argv[0] = "/usr/bin/orca"
runpy.run_path("/usr/bin/orca", run_name="__main__")
