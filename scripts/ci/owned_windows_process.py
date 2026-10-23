"""Bound a qualification subprocess tree before any child code executes.

Uses the existing Windows qualification ABI definitions; it creates no principal,
ACL, firewall rule, service or product dependency. Output stays in bounded pipes.
"""
import ctypes as c
from ctypes import wintypes as w
from contextlib import ExitStack
import os
import subprocess
import threading
import time

from run_offline_lifecycle import require
from windows_offline import (Information, JobLimits, P, Startup, U32, api, boolean,
                             close, kernel, quiesce_job)


def run_owned(arguments, *, env, timeout=600, limit=8 * 1024 * 1024):
    import msvcrt
    streams = [bytearray(), bytearray()]
    exceeded = threading.Event()
    readers = []
    with ExitStack() as stack:
        job = api(kernel, "CreateJobObjectW", P, P, w.LPCWSTR)(None, None)
        boolean(job, "create capacity process job")
        # Child handles and output pipes must stay valid until the tree quiesces.
        limits = JobLimits()
        limits.basic.flags = 0x2000
        try:
            boolean(api(kernel, "SetInformationJobObject", w.BOOL, P, U32, P, U32)(
                job, 9, c.byref(limits), c.sizeof(limits)), "bind capacity job lifetime")
            startup, information = Startup(), Information()
            startup.size, startup.flags, startup.show = c.sizeof(startup), 0x101, 0
            null = stack.enter_context(open(os.devnull, "rb"))
            startup.stdin = msvcrt.get_osfhandle(null.fileno())
            os.set_handle_inheritable(startup.stdin, True)
            write_fds = []
            for index, field in enumerate(["stdout", "stderr"]):
                read_fd, write_fd = os.pipe()
                source = stack.enter_context(os.fdopen(read_fd, "rb", buffering=0))
                write_fds.append(write_fd)
                handle = msvcrt.get_osfhandle(write_fd)
                os.set_handle_inheritable(handle, True)
                setattr(startup, field, handle)

                def capture(pipe=source, destination=streams[index]):
                    try:
                        while chunk := pipe.read(65536):
                            if len(destination) + len(chunk) > limit:
                                exceeded.set()
                                return
                            destination.extend(chunk)
                    except OSError:
                        exceeded.set()

                reader = threading.Thread(target=capture, daemon=True)
                readers.append(reader)
                reader.start()
            command = c.create_unicode_buffer(subprocess.list2cmdline([str(a) for a in arguments]))
            block = c.create_unicode_buffer("\0".join(f"{k}={v}" for k, v in sorted(env.items())) + "\0\0")
            try:
                boolean(api(kernel, "CreateProcessW", w.BOOL, w.LPCWSTR, w.LPWSTR,
                            P, P, w.BOOL, U32, P, w.LPCWSTR, P, P)(
                    str(arguments[0]), command, None, None, True, 0x08000404, block, None,
                    c.byref(startup), c.byref(information)), "create suspended capacity child")
            finally:
                for descriptor in write_fds:
                    os.close(descriptor)
                os.set_handle_inheritable(startup.stdin, False)
            stack.callback(close, information.process)
            stack.callback(close, information.thread)
            # Assignment failure must also terminate the suspended child.
            stack.callback(api(kernel, "TerminateProcess", w.BOOL, P, U32), information.process, 1)
            boolean(api(kernel, "AssignProcessToJobObject", w.BOOL, P, P)(job, information.process),
                    "assign capacity child before execution")
            require(api(kernel, "ResumeThread", U32, P)(information.thread) != 0xFFFFFFFF,
                    "resume capacity child")
            wait = api(kernel, "WaitForSingleObject", U32, P, U32)
            deadline = time.monotonic() + timeout
            while (state := wait(information.process, 50)) == 258:
                require(not exceeded.is_set(), "capacity child output exceeded bound")
                require(time.monotonic() < deadline, "capacity child exceeded deadline")
            require(state == 0, "capacity child wait failed")
            code = U32()
            boolean(api(kernel, "GetExitCodeProcess", w.BOOL, P, P)(information.process, c.byref(code)),
                    "read capacity child exit")
        finally:
            quiesce_job(job)
            for reader in readers:
                reader.join(timeout=2)
        require(not exceeded.is_set() and not any(r.is_alive() for r in readers),
                "capacity output incomplete or exceeded bound")
    return subprocess.CompletedProcess(arguments, code.value,
                                       streams[0].decode("utf-8", errors="replace"),
                                       streams[1].decode("utf-8", errors="replace"))
