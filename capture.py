"""Renode capture sequence for the Game Boy step debugger (not yet run).

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app gameboy --script ../precursor-gameboy/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
"""
import os


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    shot("01_demos.png")
    ctl.timed_key("Down", after=0.5)
    ctl.timed_key("Down", after=0.5)       # Fibonacci
    ctl.inject_line("")                    # Enter: load it
    shot("02_loaded.png")
    ctl.inject_string(" ")                 # space: run to HALT
    ctl.sleep(1.0)
    shot("03_halted.png")
    ctl.inject_string("m")                 # memory viewer at $C000: 01 01 02 03 05 08 0D 15
    ctl.sleep(1.0)
    shot("04_memory.png")
    ctl.timed_key("BackSpace", after=1.0)  # back to the debugger
    ctl.timed_key("BackSpace", after=1.0)  # back to the demo list
    ctl.timed_key("F4", after=1.0)         # leave the app
    return failures
