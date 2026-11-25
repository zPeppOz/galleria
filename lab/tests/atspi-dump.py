#!/usr/bin/env python3
"""Print the AT-SPI accessibility tree of the Galleria window, one line per node (lab/tests/gui-a11y.sh, ui/e2e/*.a11y.ts).

    atspi-dump.py [--app NAME] [--max-depth N]

Each line: depth, role, name, states, description and relations, separated by TAB. States are the AT-SPI nicks
(`focused`, `showing`, `checked`, `expanded`, ...). Relations are `type->name` of the target, comma separated.
The application is the first one on the AT-SPI desktop whose name contains NAME (default: galleria). Exit 3 when
there is none, so a caller can wait for the window.
"""
import argparse
import sys

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402


def states_of(node):
    try:
        return sorted(s.value_nick for s in node.get_state_set().get_states())
    except Exception:  # an object that vanishes while we walk the tree
        return []


def relations_of(node):
    out = []
    try:
        for rel in node.get_relation_set():
            kind = rel.get_relation_type().value_nick
            for i in range(rel.get_n_targets()):
                target = rel.get_target(i)
                out.append(f"{kind}->{target.get_name() or target.get_role_name()}")
    except Exception:
        pass
    return out


def walk(node, depth, max_depth, lines):
    try:
        role = node.get_role_name()
        name = node.get_name() or ""
        description = node.get_description() or ""
    except Exception:
        return
    clean = lambda text: text.replace("\t", " ").replace("\n", " ")
    lines.append("\t".join([str(depth), role, clean(name), ",".join(states_of(node)), clean(description), ",".join(relations_of(node))]))
    if depth >= max_depth:
        return
    try:
        count = node.get_child_count()
    except Exception:
        return
    for i in range(count):
        try:
            child = node.get_child_at_index(i)
        except Exception:
            continue
        if child is not None:
            walk(child, depth + 1, max_depth, lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--app", default="galleria")
    parser.add_argument("--max-depth", type=int, default=40)
    args = parser.parse_args()
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(i)
        if app is not None and args.app.lower() in (app.get_name() or "").lower():
            lines = []
            walk(app, 0, args.max_depth, lines)
            print("\n".join(lines))
            return 0
    print(f"no application named like {args.app!r} on the AT-SPI desktop", file=sys.stderr)
    return 3


if __name__ == "__main__":
    sys.exit(main())
