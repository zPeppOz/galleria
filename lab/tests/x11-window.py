#!/usr/bin/env python3
"""The window of Galleria seen and closed the way a window manager would (plan B6, T3, S60).

Under Xvfb there is no window manager, so nobody sends the request of closing that the «X» of a title bar sends. This does: a
`WM_DELETE_WINDOW` client message to the top-level window whose name is «Galleria», which GTK turns into `delete-event` and Tauri into
`CloseRequested`. It reads whether the window is mapped too, from the X server. Only `libX11` through `ctypes`: nothing to install.

    x11-window.py close      sends the request of closing to every window named «Galleria»; exits 1 when there is none
    x11-window.py mapped     prints `true` or `false`: a window named «Galleria» is mapped (shown); `none` when there is no window
    x11-window.py wait-mapped [true|false] [seconds]    waits until that is the case (default: true, 10 s); exits 1 when it never is
"""
import ctypes
import ctypes.util
import sys
import time

NAME = "Galleria"
x11 = ctypes.CDLL(ctypes.util.find_library("X11") or "libX11.so.6")

Window = ctypes.c_ulong
Atom = ctypes.c_ulong

x11.XOpenDisplay.restype = ctypes.c_void_p
x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
x11.XDefaultRootWindow.restype = Window
x11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
x11.XInternAtom.restype = Atom
x11.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
x11.XQueryTree.argtypes = [ctypes.c_void_p, Window, ctypes.POINTER(Window), ctypes.POINTER(Window), ctypes.POINTER(ctypes.POINTER(Window)), ctypes.POINTER(ctypes.c_uint)]
x11.XFetchName.argtypes = [ctypes.c_void_p, Window, ctypes.POINTER(ctypes.c_char_p)]
x11.XFree.argtypes = [ctypes.c_void_p]
x11.XSendEvent.argtypes = [ctypes.c_void_p, Window, ctypes.c_int, ctypes.c_long, ctypes.c_void_p]
x11.XFlush.argtypes = [ctypes.c_void_p]
x11.XGetWindowProperty.argtypes = [
    ctypes.c_void_p, Window, Atom, ctypes.c_long, ctypes.c_long, ctypes.c_int, Atom,
    ctypes.POINTER(Atom), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong),
    ctypes.POINTER(ctypes.POINTER(ctypes.c_ubyte)),
]


class XWindowAttributes(ctypes.Structure):
    # the head of the structure, up to `map_state`: the rest is not read
    _fields_ = [
        ("x", ctypes.c_int), ("y", ctypes.c_int), ("width", ctypes.c_int), ("height", ctypes.c_int),
        ("border_width", ctypes.c_int), ("depth", ctypes.c_int), ("visual", ctypes.c_void_p), ("root", Window),
        ("klass", ctypes.c_int), ("bit_gravity", ctypes.c_int), ("win_gravity", ctypes.c_int), ("backing_store", ctypes.c_int),
        ("backing_planes", ctypes.c_ulong), ("backing_pixel", ctypes.c_ulong), ("save_under", ctypes.c_int),
        ("colormap", ctypes.c_ulong), ("map_installed", ctypes.c_int), ("map_state", ctypes.c_int),
        ("all_event_masks", ctypes.c_long), ("your_event_mask", ctypes.c_long), ("do_not_propagate_mask", ctypes.c_long),
        ("override_redirect", ctypes.c_int), ("screen", ctypes.c_void_p),
    ]


x11.XGetWindowAttributes.argtypes = [ctypes.c_void_p, Window, ctypes.POINTER(XWindowAttributes)]


class ClientMessage(ctypes.Structure):
    _fields_ = [
        ("type", ctypes.c_int), ("serial", ctypes.c_ulong), ("send_event", ctypes.c_int), ("display", ctypes.c_void_p),
        ("window", Window), ("message_type", Atom), ("format", ctypes.c_int), ("data", ctypes.c_long * 5),
        ("pad", ctypes.c_long * 14),
    ]


CLIENT_MESSAGE = 33
IS_VIEWABLE = 2


def display():
    found = x11.XOpenDisplay(None)
    if not found:
        raise SystemExit("cannot open the X display")
    return found


def windows(dpy):
    """The windows named `NAME`: the top-level ones (children of the root), and the windows inside them that carry the name."""
    root = x11.XDefaultRootWindow(dpy)
    found = []

    def walk(window, depth):
        root_return, parent_return, children, count = Window(), Window(), ctypes.POINTER(Window)(), ctypes.c_uint()
        if not x11.XQueryTree(dpy, window, ctypes.byref(root_return), ctypes.byref(parent_return), ctypes.byref(children), ctypes.byref(count)):
            return
        kids = [children[i] for i in range(count.value)]
        if children:
            x11.XFree(children)
        for kid in kids:
            name = ctypes.c_char_p()
            if x11.XFetchName(dpy, kid, ctypes.byref(name)) and name.value:
                if name.value.decode("utf8", "replace") == NAME:
                    found.append(kid)
                x11.XFree(name)
            if depth < 2:
                walk(kid, depth + 1)

    walk(root, 0)
    return found


def mapped(dpy, window):
    attributes = XWindowAttributes()
    return bool(x11.XGetWindowAttributes(dpy, window, ctypes.byref(attributes))) and attributes.map_state == IS_VIEWABLE


def close():
    dpy = display()
    targets = windows(dpy)
    if not targets:
        return 1
    root = x11.XDefaultRootWindow(dpy)
    protocols, delete = x11.XInternAtom(dpy, b"WM_PROTOCOLS", 0), x11.XInternAtom(dpy, b"WM_DELETE_WINDOW", 0)
    for window in targets:
        event = ClientMessage()
        event.type, event.window, event.message_type, event.format, event.display = CLIENT_MESSAGE, window, protocols, 32, dpy
        event.data[0], event.data[1] = delete, 0
        x11.XSendEvent(dpy, window, 0, 0, ctypes.byref(event))
    x11.XFlush(dpy)
    _ = root
    return 0


def state():
    dpy = display()
    targets = windows(dpy)
    if not targets:
        return "none"
    return "true" if any(mapped(dpy, window) for window in targets) else "false"


def main(argv):
    command = argv[1] if len(argv) > 1 else ""
    if command == "close":
        sys.exit(close())
    elif command == "mapped":
        print(state())
    elif command == "wait-mapped":
        want = (argv[2] if len(argv) > 2 else "true")
        deadline = time.time() + (float(argv[3]) if len(argv) > 3 else 10.0)
        while time.time() < deadline:
            if state() == want:
                return
            time.sleep(0.1)
        print(f"the window is {state()!r}, not {want!r}", file=sys.stderr)
        sys.exit(1)
    else:
        print(__doc__, file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main(sys.argv)
