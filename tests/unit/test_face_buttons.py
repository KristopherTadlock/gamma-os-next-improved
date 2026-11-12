"""The RetroArch face-button swap in 10-update-emulators.sh (its embedded Python, run as the mod runs
it): the stock binds by print become binds by position, and a second run leaves them alone."""
import re
import sys

from tests import util

STOCK = {"a": "96", "b": "97", "x": "99", "y": "100"}
SWAPPED = {"a": "97", "b": "96", "x": "100", "y": "99"}


def swap_script():
    text = util.slurp(f"{util.MODS}/10-update-emulators.sh", text=True)
    m = re.search(r"retrogame_joypad\.cfg\" \\\n.*?<<'PY'\n(.*?)\nPY\n", text, re.S)
    assert m, "no face-button swap in 10-update-emulators.sh"
    return m.group(1)


def cfg(prefix, binds, extra=""):
    return "".join(f'{prefix}{k}_btn = "{v}"\n' for k, v in binds.items()) + 'input_l_btn = "102"\n' + extra


class RetroArchFaceButtons(util.TestCase):
    def run_swap(self, joypad, retroarch):
        a = self.write("retrogame_joypad.cfg", joypad)
        b = self.write("retroarch.cfg", retroarch)
        proc = util.run([sys.executable, "-I", "-", a, b], input=swap_script())
        return proc, util.slurp(a, text=True), util.slurp(b, text=True)

    def test_swaps_once(self):
        proc, joypad, ra = self.run_swap(cfg("input_", STOCK), cfg("input_player1_", STOCK))
        self.assertOk(proc)
        self.assertEqual(joypad, cfg("input_", SWAPPED))
        self.assertEqual(ra, cfg("input_player1_", SWAPPED))
        proc, joypad2, ra2 = self.run_swap(joypad, ra)
        self.assertOk(proc)
        self.assertIn("already by position", proc.stdout)
        self.assertEqual((joypad2, ra2), (joypad, ra))

    def test_unexpected_binds(self):
        odd = dict(STOCK, b="23")
        proc, joypad, _ = self.run_swap(cfg("input_", odd), cfg("input_player1_", STOCK))
        self.assertFails(proc, "unexpected face-button binds")
        self.assertEqual(joypad, cfg("input_", odd))
        missing = {k: v for k, v in STOCK.items() if k != "y"}
        self.assertFails(self.run_swap(cfg("input_", missing), cfg("input_player1_", STOCK))[0],
                         "3 face-button binds, expected 4")
