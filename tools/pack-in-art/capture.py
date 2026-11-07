#!/usr/bin/env python3
"""Runs a game in a libretro core with no window and saves frames as PNGs.

  capture.py CORE.so GAME OUT_PREFIX SCRIPT

SCRIPT is a list of steps separated by spaces, run in order:
  <n>            run n frames with no buttons held
  <button>:<n>   hold a button (a, b, x, y, l, r, start, select, up, down,
                 left, right) for n frames, then release it for 10
  shot           save the current frame as OUT_PREFIX-<k>.png

Needs Pillow. Only cores that draw in software work (no OpenGL).
"""

import ctypes as C
import os
import sys

from PIL import Image

BUTTONS = {"b": 0, "y": 1, "select": 2, "start": 3, "up": 4, "down": 5, "left": 6, "right": 7,
           "a": 8, "x": 9, "l": 10, "r": 11}

ENV_GET_CAN_DUPE = 3
ENV_GET_SYSTEM_DIRECTORY = 9
ENV_SET_PIXEL_FORMAT = 10
ENV_GET_VARIABLE = 15
ENV_GET_VARIABLE_UPDATE = 17
ENV_GET_SAVE_DIRECTORY = 31
ENV_GET_CORE_OPTIONS_VERSION = 52
ENV_EXPERIMENTAL = 0x10000


class GameInfo(C.Structure):
    _fields_ = [("path", C.c_char_p), ("data", C.c_void_p), ("size", C.c_size_t), ("meta", C.c_char_p)]


class SystemInfo(C.Structure):
    _fields_ = [("library_name", C.c_char_p), ("library_version", C.c_char_p),
                ("valid_extensions", C.c_char_p), ("need_fullpath", C.c_bool), ("block_extract", C.c_bool)]


class Variable(C.Structure):
    _fields_ = [("key", C.c_char_p), ("value", C.c_char_p)]


ENV_CB = C.CFUNCTYPE(C.c_bool, C.c_uint, C.c_void_p)
VIDEO_CB = C.CFUNCTYPE(None, C.c_void_p, C.c_uint, C.c_uint, C.c_size_t)
AUDIO_CB = C.CFUNCTYPE(None, C.c_int16, C.c_int16)
AUDIO_BATCH_CB = C.CFUNCTYPE(C.c_size_t, C.c_void_p, C.c_size_t)
POLL_CB = C.CFUNCTYPE(None)
INPUT_CB = C.CFUNCTYPE(C.c_int16, C.c_uint, C.c_uint, C.c_uint, C.c_uint)


def main():
    core_path, game, prefix, script = sys.argv[1:5]
    work = os.path.abspath(prefix + "-sys")
    os.makedirs(work, exist_ok=True)
    work_c = C.c_char_p(work.encode())
    options = dict(o.split("=", 1) for o in os.environ.get("CORE_OPTIONS", "").split(";") if "=" in o)
    option_values = {k: C.c_char_p(v.encode()) for k, v in options.items()}
    state = {"fmt": 0, "frame": None, "held": set()}

    def env(cmd, data):
        cmd &= ~ENV_EXPERIMENTAL
        if cmd in (ENV_GET_SYSTEM_DIRECTORY, ENV_GET_SAVE_DIRECTORY):
            C.cast(data, C.POINTER(C.c_char_p))[0] = work_c.value
            return True
        if cmd == ENV_SET_PIXEL_FORMAT:
            state["fmt"] = C.cast(data, C.POINTER(C.c_int))[0]
            return True
        if cmd == ENV_GET_CAN_DUPE:
            C.cast(data, C.POINTER(C.c_bool))[0] = True
            return True
        if cmd == ENV_GET_VARIABLE:
            var = C.cast(data, C.POINTER(Variable))[0]
            key = var.key.decode() if var.key else ""
            if key in option_values:
                C.cast(data, C.POINTER(Variable))[0].value = option_values[key].value
                return True
            return False
        if cmd == ENV_GET_VARIABLE_UPDATE:
            C.cast(data, C.POINTER(C.c_bool))[0] = False
            return True
        if cmd == ENV_GET_CORE_OPTIONS_VERSION:
            C.cast(data, C.POINTER(C.c_uint))[0] = 0
            return True
        return False

    def video(data, width, height, pitch):
        if not data:
            return
        raw = C.string_at(data, pitch * height)
        if state["fmt"] == 1:
            img = Image.frombuffer("RGBA", (width, height), raw, "raw", "BGRA", pitch, 1)
            img = img.convert("RGB")
        elif state["fmt"] == 2:
            img = Image.frombuffer("RGB", (width, height), raw, "raw", "BGR;16", pitch, 1)
        else:
            img = Image.frombuffer("RGB", (width, height), raw, "raw", "BGR;15", pitch, 1)
        state["frame"] = img.copy()

    def input_state(port, device, index, button):
        return 1 if port == 0 and device == 1 and button in state["held"] else 0

    callbacks = [ENV_CB(env), VIDEO_CB(video), AUDIO_CB(lambda left, right: None),
                 AUDIO_BATCH_CB(lambda data, frames: frames), POLL_CB(lambda: None), INPUT_CB(input_state)]
    core = C.CDLL(os.path.abspath(core_path))
    core.retro_set_environment(callbacks[0])
    core.retro_init()
    core.retro_set_video_refresh(callbacks[1])
    core.retro_set_audio_sample(callbacks[2])
    core.retro_set_audio_sample_batch(callbacks[3])
    core.retro_set_input_poll(callbacks[4])
    core.retro_set_input_state(callbacks[5])

    info = SystemInfo()
    core.retro_get_system_info(C.byref(info))
    game_path = os.path.abspath(game).encode()
    data = None if info.need_fullpath else open(game, "rb").read()
    buf = C.create_string_buffer(data, len(data)) if data else None
    game_info = GameInfo(game_path, C.cast(buf, C.c_void_p) if buf else None, len(data) if data else 0, None)
    core.retro_load_game.restype = C.c_bool
    if not core.retro_load_game(C.byref(game_info)):
        sys.exit(f"{core_path} couldn't load {game}")

    def run(frames):
        for _ in range(frames):
            core.retro_run()

    shots = 0
    for step in script.split():
        if step == "shot":
            shots += 1
            out = f"{prefix}-{shots}.png"
            state["frame"].save(out)
            print(out, state["frame"].size)
        elif ":" in step:
            button, frames = step.split(":")
            state["held"] = {BUTTONS[button]}
            run(int(frames))
            state["held"] = set()
            run(10)
        else:
            run(int(step))


if __name__ == "__main__":
    main()
